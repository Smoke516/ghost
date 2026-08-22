//! Logical topology of the server list: what reaches what.
//!
//! This is deliberately *not* a geographic map. Placing hosts on a world map
//! needs lat/long, which means either bundling a GeoIP database or shipping the
//! user's private hostnames to a third-party API — and most SSH lists are full
//! of LAN names and VPN endpoints that have no meaningful location anyway.
//!
//! What a list genuinely hides is routing: which hosts are reached through a
//! bastion, and therefore which ones all go dark together when that bastion
//! does. That structure comes free from `ProxyJump`.

use crate::models::ServerConnection;

/// One rendered line in the topology view.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    /// A bastion, or the synthetic "Direct" bucket.
    Group {
        label: String,
        /// Index into the flat host list, when the bastion is itself a
        /// configured server (so it can be selected and connected to).
        server: Option<usize>,
        /// How many hosts sit behind it.
        children: usize,
        /// False for the synthetic "Direct" bucket, which is a heading rather
        /// than a real jump host and must not be described as one.
        is_bastion: bool,
    },
    /// A host under the group above it.
    Host {
        server: usize,
        /// True for the last child, so the renderer can draw the elbow.
        last: bool,
    },
    /// Blank separator between groups.
    Spacer,
}

/// A group of hosts sharing one bastion.
struct Group {
    label: String,
    server: Option<usize>,
    members: Vec<usize>,
}

/// Does this connection identify the given jump target?
///
/// `ProxyJump` names an ssh_config alias, which Ghost stores as the display
/// name on import, but users also hand-write the hostname. Match either, plus
/// `user@host`, case-insensitively.
fn identifies(conn: &ServerConnection, target: &str) -> bool {
    let target = target.trim().to_lowercase();
    conn.name.to_lowercase() == target
        || conn.host.to_lowercase() == target
        || format!("{}@{}", conn.username, conn.host).to_lowercase() == target
}

/// Build the display rows for a set of connections.
///
/// `servers` is expected to be the same slice the caller will index into when
/// rendering, so indices stay valid.
pub fn build(servers: &[&ServerConnection]) -> Vec<Row> {
    let mut direct: Vec<usize> = Vec::new();
    // Grouped by resolved bastion label, preserving first-seen order.
    let mut groups: Vec<Group> = Vec::new();

    for (i, conn) in servers.iter().enumerate() {
        let Some(jump) = conn.proxy_jump.as_deref().filter(|j| !j.trim().is_empty()) else {
            direct.push(i);
            continue;
        };

        // Prefer the bastion's own display name when it is a known server, so
        // `ProxyJump 10.0.0.1` and `ProxyJump bastion` collapse into one group.
        let bastion = servers.iter().position(|s| identifies(s, jump));
        let label = bastion
            .map(|b| servers[b].name.clone())
            .unwrap_or_else(|| jump.to_string());

        match groups.iter_mut().find(|g| g.label == label) {
            Some(g) => g.members.push(i),
            None => groups.push(Group {
                label,
                server: bastion,
                members: vec![i],
            }),
        }
    }

    // A bastion that is itself in the list should not also appear as a direct
    // host — it is already the head of its own group.
    let bastion_indices: Vec<usize> = groups.iter().filter_map(|g| g.server).collect();
    direct.retain(|i| !bastion_indices.contains(i));

    let mut rows = Vec::new();

    for group in &groups {
        rows.push(Row::Group {
            label: group.label.clone(),
            server: group.server,
            children: group.members.len(),
            is_bastion: true,
        });
        let last = group.members.len().saturating_sub(1);
        for (n, &m) in group.members.iter().enumerate() {
            rows.push(Row::Host {
                server: m,
                last: n == last,
            });
        }
        rows.push(Row::Spacer);
    }

    if !direct.is_empty() {
        // Only label the direct bucket when there is something to contrast it
        // with; a flat list needs no heading.
        if !groups.is_empty() {
            rows.push(Row::Group {
                label: "Direct".to_string(),
                server: None,
                children: direct.len(),
                is_bastion: false,
            });
        }
        let last = direct.len().saturating_sub(1);
        for (n, &d) in direct.iter().enumerate() {
            rows.push(Row::Host {
                server: d,
                last: n == last && !groups.is_empty(),
            });
        }
    }

    while matches!(rows.last(), Some(Row::Spacer)) {
        rows.pop();
    }

    rows
}

/// One hop on the path from your machine to a host.
#[derive(Debug, Clone, PartialEq)]
pub struct Hop {
    pub label: String,
    /// Index into the server slice, when this hop is a configured host.
    pub server: Option<usize>,
}

/// The full path taken to reach `index`, outermost hop first, ending with the
/// host itself.
///
/// Bastions can chain (`web` jumps via `inner`, which jumps via `edge`), and a
/// mis-written config can make them cycle, so the walk is depth-limited and
/// tracks what it has already visited.
pub fn jump_chain(servers: &[&ServerConnection], index: usize) -> Vec<Hop> {
    let mut chain = vec![Hop {
        label: servers[index].name.clone(),
        server: Some(index),
    }];
    let mut seen = vec![index];
    let mut current = index;

    // OpenSSH itself caps jump depth; anything beyond a handful is pathological.
    for _ in 0..8 {
        let Some(jump) = servers[current]
            .proxy_jump
            .as_deref()
            .filter(|j| !j.trim().is_empty())
        else {
            break;
        };

        match servers.iter().position(|s| identifies(s, jump)) {
            Some(next) if !seen.contains(&next) => {
                chain.push(Hop {
                    label: servers[next].name.clone(),
                    server: Some(next),
                });
                seen.push(next);
                current = next;
            }
            // Either a bastion we have no entry for, or a cycle. Either way the
            // walk stops; record the name so the path is still truthful.
            Some(_) => break,
            None => {
                chain.push(Hop {
                    label: jump.to_string(),
                    server: None,
                });
                break;
            }
        }
    }

    chain.reverse();
    chain
}

/// The group heading that governs `row`, so a scrolled view can pin it.
pub fn group_of(rows: &[Row], row: usize) -> Option<usize> {
    rows[..=row.min(rows.len().saturating_sub(1))]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, r)| matches!(r, Row::Group { .. }))
        .map(|(i, _)| i)
}

/// Row indices that can be selected, i.e. those backed by a real server.
pub fn selectable(rows: &[Row]) -> Vec<usize> {
    // A bastion that itself sits behind another bastion appears twice: once as
    // a host under its parent, and once as the heading of its own group. That
    // duplication is correct in the tree — it shows both relationships — but it
    // should not give the same server two cursor stops.
    let as_host: Vec<usize> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Host { server, .. } => Some(*server),
            _ => None,
        })
        .collect();

    rows.iter()
        .enumerate()
        .filter(|(_, r)| match r {
            Row::Host { .. } => true,
            Row::Group { server, .. } => server.is_some_and(|srv| !as_host.contains(&srv)),
            Row::Spacer => false,
        })
        .map(|(i, _)| i)
        .collect()
}

/// The server a row points at, if any.
pub fn server_of(row: &Row) -> Option<usize> {
    match row {
        Row::Host { server, .. } => Some(*server),
        Row::Group { server, .. } => *server,
        Row::Spacer => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(name: &str, host: &str, jump: Option<&str>) -> ServerConnection {
        let mut c = ServerConnection::new(name.into(), host.into(), 22, "me".into());
        c.proxy_jump = jump.map(|j| j.to_string());
        c
    }

    fn rows_of(owned: &[ServerConnection]) -> Vec<Row> {
        let refs: Vec<&ServerConnection> = owned.iter().collect();
        build(&refs)
    }

    #[test]
    fn a_flat_list_needs_no_grouping() {
        let owned = vec![conn("a", "a.example", None), conn("b", "b.example", None)];
        let rows = rows_of(&owned);
        // No headings at all when nothing jumps.
        assert!(!rows.iter().any(|r| matches!(r, Row::Group { .. })));
        assert_eq!(
            rows.iter()
                .filter(|r| matches!(r, Row::Host { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn hosts_group_under_their_bastion() {
        let owned = vec![
            conn("bastion", "bastion.corp", None),
            conn("web-01", "10.0.1.11", Some("bastion")),
            conn("web-02", "10.0.1.12", Some("bastion")),
        ];
        let rows = rows_of(&owned);
        match &rows[0] {
            Row::Group {
                label,
                children,
                server,
                ..
            } => {
                assert_eq!(label, "bastion");
                assert_eq!(*children, 2);
                assert_eq!(*server, Some(0), "bastion should be selectable");
            }
            other => panic!("expected a group, got {:?}", other),
        }
        assert!(matches!(rows[1], Row::Host { last: false, .. }));
        assert!(matches!(rows[2], Row::Host { last: true, .. }));
    }

    #[test]
    fn a_bastion_is_not_also_listed_as_direct() {
        let owned = vec![
            conn("bastion", "bastion.corp", None),
            conn("web", "10.0.1.11", Some("bastion")),
        ];
        let rows = rows_of(&owned);
        let host_rows: Vec<usize> = rows.iter().filter_map(server_of).collect();
        assert_eq!(host_rows.iter().filter(|&&i| i == 0).count(), 1);
    }

    #[test]
    fn jump_by_hostname_and_by_alias_collapse_together() {
        // One user writes `ProxyJump bastion`, another `ProxyJump bastion.corp`.
        let owned = vec![
            conn("bastion", "bastion.corp", None),
            conn("a", "10.0.0.1", Some("bastion")),
            conn("b", "10.0.0.2", Some("bastion.corp")),
        ];
        let rows = rows_of(&owned);
        assert_eq!(
            rows.iter()
                .filter(|r| matches!(r, Row::Group { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn an_unknown_bastion_still_forms_a_group() {
        // Jumping through a host that is not itself configured is normal.
        let owned = vec![conn("a", "10.0.0.1", Some("ghost-bastion"))];
        let rows = rows_of(&owned);
        match &rows[0] {
            Row::Group { label, server, .. } => {
                assert_eq!(label, "ghost-bastion");
                assert_eq!(*server, None, "not selectable: we have no such server");
            }
            other => panic!("expected a group, got {:?}", other),
        }
    }

    #[test]
    fn direct_hosts_are_bucketed_when_groups_exist() {
        let owned = vec![
            conn("bastion", "bastion.corp", None),
            conn("web", "10.0.1.11", Some("bastion")),
            conn("home", "home.lan", None),
        ];
        let rows = rows_of(&owned);
        assert!(rows
            .iter()
            .any(|r| matches!(r, Row::Group { label, .. } if label == "Direct")));
    }

    #[test]
    fn only_real_servers_are_selectable() {
        let owned = vec![conn("a", "10.0.0.1", Some("unknown-bastion"))];
        let rows = rows_of(&owned);
        let sel = selectable(&rows);
        // The synthetic group heading is skipped; only the host is reachable.
        assert_eq!(sel.len(), 1);
        assert_eq!(server_of(&rows[sel[0]]), Some(0));
    }

    #[test]
    fn the_direct_bucket_is_not_a_bastion() {
        let owned = vec![
            conn("bastion", "bastion.corp", None),
            conn("web", "10.0.1.11", Some("bastion")),
            conn("home", "home.lan", None),
        ];
        let rows = rows_of(&owned);
        let direct = rows
            .iter()
            .find_map(|r| match r {
                Row::Group {
                    label, is_bastion, ..
                } if label == "Direct" => Some(*is_bastion),
                _ => None,
            })
            .expect("expected a Direct bucket");
        assert!(!direct, "Direct is a heading, not a jump host");
    }

    #[test]
    fn jump_chain_walks_outward_from_the_host() {
        let owned = [
            conn("edge", "edge.corp", None),
            conn("inner", "inner.corp", Some("edge")),
            conn("web", "10.0.0.1", Some("inner")),
        ];
        let refs: Vec<&ServerConnection> = owned.iter().collect();
        let chain = jump_chain(&refs, 2);
        let labels: Vec<&str> = chain.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(labels, vec!["edge", "inner", "web"]);
    }

    #[test]
    fn a_direct_host_has_a_single_hop() {
        let owned = [conn("solo", "solo.lan", None)];
        let refs: Vec<&ServerConnection> = owned.iter().collect();
        assert_eq!(jump_chain(&refs, 0).len(), 1);
    }

    #[test]
    fn jump_chain_records_an_unconfigured_bastion() {
        let owned = [conn("web", "10.0.0.1", Some("mystery"))];
        let refs: Vec<&ServerConnection> = owned.iter().collect();
        let chain = jump_chain(&refs, 0);
        assert_eq!(chain[0].label, "mystery");
        assert_eq!(chain[0].server, None);
        assert_eq!(chain[1].label, "web");
    }

    #[test]
    fn a_cyclic_jump_config_terminates() {
        // a jumps via b, b jumps via a. Without cycle tracking this never ends.
        let owned = [
            conn("a", "a.corp", Some("b")),
            conn("b", "b.corp", Some("a")),
        ];
        let refs: Vec<&ServerConnection> = owned.iter().collect();
        let chain = jump_chain(&refs, 0);
        assert!(chain.len() <= 3, "cycle produced {} hops", chain.len());
    }

    #[test]
    fn group_of_finds_the_governing_heading() {
        let owned = vec![
            conn("bastion", "bastion.corp", None),
            conn("web-01", "10.0.1.11", Some("bastion")),
            conn("web-02", "10.0.1.12", Some("bastion")),
        ];
        let rows = rows_of(&owned);
        // Row 0 is the group; rows 1 and 2 are its hosts.
        assert_eq!(group_of(&rows, 2), Some(0));
        assert_eq!(group_of(&rows, 0), Some(0));
    }

    #[test]
    fn a_chained_bastion_gets_one_cursor_stop_not_two() {
        // bastion-eu is reached via edge-gw and is itself a bastion, so it
        // shows up as both a host row and a group heading.
        let owned = [
            conn("edge-gw", "edge.corp", None),
            conn("bastion-eu", "eu.corp", Some("edge-gw")),
            conn("web", "10.0.0.1", Some("bastion-eu")),
        ];
        let rows = rows_of(&owned);
        let stops: Vec<Option<usize>> = selectable(&rows)
            .iter()
            .map(|&r| server_of(&rows[r]))
            .collect();
        let eu = stops.iter().filter(|s| **s == Some(1)).count();
        assert_eq!(eu, 1, "bastion-eu should be selectable exactly once");
    }

    #[test]
    fn an_unprobed_bastion_is_not_a_known_failure() {
        // Documents the rule the path panel relies on: Unknown means "not
        // checked yet", which must not be reported as a blocked route.
        use crate::models::HealthStatus;
        let mut owned = [
            conn("gw", "gw.corp", None),
            conn("node", "10.0.0.1", Some("gw")),
        ];
        owned[0].health_status = HealthStatus::Unknown;
        assert!(!matches!(owned[0].health_status, HealthStatus::Offline));

        owned[0].health_status = HealthStatus::Offline;
        assert!(matches!(owned[0].health_status, HealthStatus::Offline));
    }

    #[test]
    fn empty_input_produces_no_rows() {
        assert!(rows_of(&[]).is_empty());
    }
}
