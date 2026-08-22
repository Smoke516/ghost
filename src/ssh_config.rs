//! Minimal `~/.ssh/config` parser.
//!
//! Deliberately not a full OpenSSH implementation: it understands the subset
//! that maps cleanly onto a Ghost connection (`Host`, `HostName`, `User`,
//! `Port`, `IdentityFile`, and `Include`). Anything it doesn't understand is
//! ignored rather than treated as an error — a config Ghost can't fully model
//! should still yield the hosts it *can* model.

use crate::models::{AuthMethod, ServerConnection};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A host block distilled down to what Ghost stores.
#[derive(Debug, Clone, PartialEq)]
pub struct SshHost {
    /// The `Host` alias, used as the display name.
    pub alias: String,
    pub hostname: String,
    pub user: Option<String>,
    pub port: u16,
    pub identity_file: Option<String>,
}

impl SshHost {
    fn new(alias: String) -> Self {
        Self {
            alias,
            hostname: String::new(),
            user: None,
            port: 22,
            identity_file: None,
        }
    }

    /// A block is only useful to us once we know where to connect and as whom.
    fn is_complete(&self) -> bool {
        !self.hostname.is_empty() && self.user.is_some()
    }

    pub fn to_connection(&self) -> ServerConnection {
        let mut conn = ServerConnection::new(
            self.alias.clone(),
            self.hostname.clone(),
            self.port,
            self.user.clone().unwrap_or_default(),
        );
        conn.auth_method = match &self.identity_file {
            Some(path) => AuthMethod::PublicKey {
                key_path: path.clone(),
            },
            // No explicit key means the agent (or ssh's own defaults) handles
            // it — which is exactly what AuthMethod::Agent expresses.
            None => AuthMethod::Agent,
        };
        conn.description = Some(format!("Imported from ssh config ({})", self.alias));
        conn.tags = vec!["ssh-config".to_string()];
        conn
    }
}

/// Default location of the user's SSH client config.
pub fn default_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ssh").join("config"))
}

/// Patterns like `*`, `foo-?`, or negations aren't concrete hosts — they carry
/// defaults for other entries, so importing them would create junk servers.
fn is_pattern(alias: &str) -> bool {
    alias.contains('*') || alias.contains('?') || alias.starts_with('!')
}

/// Split a config line into (keyword, value). OpenSSH accepts both
/// `Key value` and `Key=value`, with arbitrary surrounding whitespace.
fn split_directive(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, value) = match line.find(['=', ' ', '\t']) {
        Some(idx) => (
            &line[..idx],
            line[idx + 1..].trim_start_matches(['=', ' ', '\t']),
        ),
        None => return None,
    };
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some((key.to_ascii_lowercase(), unquote(value).to_string()))
}

/// Values may be quoted, e.g. `IdentityFile "~/.ssh/my key"`.
fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Parse SSH config text into concrete host entries.
pub fn parse(contents: &str) -> Vec<SshHost> {
    let mut hosts = Vec::new();
    // Aliases in the current `Host` line; a single block can declare several.
    let mut current: Vec<SshHost> = Vec::new();
    // `Match` blocks are conditional; we can't evaluate their conditions, so we
    // stop collecting until the next `Host` line rather than guess.
    let mut in_unsupported_block = false;

    let flush = |current: &mut Vec<SshHost>, hosts: &mut Vec<SshHost>| {
        for host in current.drain(..) {
            if host.is_complete() {
                hosts.push(host);
            }
        }
    };

    for line in contents.lines() {
        let Some((key, value)) = split_directive(line) else {
            continue;
        };

        match key.as_str() {
            "host" => {
                flush(&mut current, &mut hosts);
                in_unsupported_block = false;
                current = value
                    .split_whitespace()
                    .filter(|alias| !is_pattern(alias))
                    .map(|alias| {
                        let mut h = SshHost::new(alias.to_string());
                        // OpenSSH defaults HostName to the alias itself.
                        h.hostname = alias.to_string();
                        h
                    })
                    .collect();
            }
            "match" => {
                flush(&mut current, &mut hosts);
                in_unsupported_block = true;
            }
            _ if in_unsupported_block || current.is_empty() => {}
            "hostname" => {
                for h in current.iter_mut() {
                    h.hostname = value.clone();
                }
            }
            "user" => {
                for h in current.iter_mut() {
                    h.user = Some(value.clone());
                }
            }
            "port" => {
                if let Ok(port) = value.parse::<u16>() {
                    for h in current.iter_mut() {
                        h.port = port;
                    }
                }
            }
            "identityfile" => {
                for h in current.iter_mut() {
                    // Only the first IdentityFile is kept; ssh tries them in
                    // order and we can pass just one via `-i`.
                    if h.identity_file.is_none() {
                        h.identity_file = Some(value.clone());
                    }
                }
            }
            _ => {}
        }
    }

    flush(&mut current, &mut hosts);
    hosts
}

/// Read and parse a config file, following `Include` directives.
///
/// `seen` guards against include cycles, which OpenSSH tolerates but which
/// would otherwise recurse forever here.
pub fn parse_file(path: &Path) -> Result<Vec<SshHost>> {
    let mut seen = HashSet::new();
    parse_file_inner(path, &mut seen, 0)
}

fn parse_file_inner(
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<Vec<SshHost>> {
    // OpenSSH caps Include nesting; a shallow limit is plenty and stops a
    // pathological config from blowing the stack.
    if depth > 8 {
        return Ok(Vec::new());
    }
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !seen.insert(canonical) {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read SSH config at {}", path.display()))?;

    let mut hosts = parse(&contents);

    // Included files contribute their own hosts.
    for line in contents.lines() {
        let Some((key, value)) = split_directive(line) else {
            continue;
        };
        if key != "include" {
            continue;
        }
        for pattern in value.split_whitespace() {
            for included in resolve_include(pattern, path) {
                if let Ok(mut more) = parse_file_inner(&included, seen, depth + 1) {
                    hosts.append(&mut more);
                }
            }
        }
    }

    Ok(hosts)
}

/// Resolve an `Include` pattern to concrete paths. Relative paths are taken
/// against the including file's directory (OpenSSH uses ~/.ssh for the user
/// config, which is the same thing in practice).
fn resolve_include(pattern: &str, parent: &Path) -> Vec<PathBuf> {
    let expanded = shellexpand::tilde(pattern).to_string();
    let candidate = PathBuf::from(&expanded);
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        parent
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(candidate)
    };

    // Globs are common (`Include config.d/*`). Expand them by listing the
    // directory rather than pulling in a glob dependency for one directive.
    if let Some(name) = candidate.file_name().and_then(|n| n.to_str()) {
        if name.contains('*') {
            let dir = candidate.parent().unwrap_or_else(|| Path::new("."));
            let prefix = name.split('*').next().unwrap_or("");
            let suffix = name.rsplit('*').next().unwrap_or("");
            let mut matches: Vec<PathBuf> = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(prefix) && n.ends_with(suffix))
                        .unwrap_or(false)
                })
                .collect();
            matches.sort();
            return matches;
        }
    }

    if candidate.is_file() {
        vec![candidate]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
# A comment
Host bastion
    HostName bastion.example.com
    User jump
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host web1 web2
    HostName shared.example.com
    User deploy

Host *
    ServerAliveInterval 60
    User should-not-leak

Host incomplete
    Port 22
"#;

    #[test]
    fn parses_basic_host_block() {
        let hosts = parse(SAMPLE);
        let bastion = hosts.iter().find(|h| h.alias == "bastion").unwrap();
        assert_eq!(bastion.hostname, "bastion.example.com");
        assert_eq!(bastion.user.as_deref(), Some("jump"));
        assert_eq!(bastion.port, 2222);
        assert_eq!(bastion.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
    }

    #[test]
    fn one_block_can_declare_several_hosts() {
        let hosts = parse(SAMPLE);
        let web: Vec<_> = hosts
            .iter()
            .filter(|h| h.alias.starts_with("web"))
            .collect();
        assert_eq!(web.len(), 2);
        assert!(web.iter().all(|h| h.hostname == "shared.example.com"));
        assert!(web.iter().all(|h| h.user.as_deref() == Some("deploy")));
    }

    #[test]
    fn wildcard_blocks_are_not_imported() {
        let hosts = parse(SAMPLE);
        assert!(!hosts.iter().any(|h| h.alias.contains('*')));
        // The `Host *` block's User must not bleed into real entries.
        assert!(!hosts
            .iter()
            .any(|h| h.user.as_deref() == Some("should-not-leak")));
    }

    #[test]
    fn blocks_without_a_user_are_skipped() {
        let hosts = parse(SAMPLE);
        assert!(!hosts.iter().any(|h| h.alias == "incomplete"));
    }

    #[test]
    fn hostname_defaults_to_the_alias() {
        let hosts = parse("Host solo\n  User me\n");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname, "solo");
    }

    #[test]
    fn accepts_equals_separated_and_quoted_values() {
        let hosts = parse("Host q\n  HostName=q.example.com\n  User=\"me\"\n  Port=2020\n");
        assert_eq!(hosts[0].hostname, "q.example.com");
        assert_eq!(hosts[0].user.as_deref(), Some("me"));
        assert_eq!(hosts[0].port, 2020);
    }

    #[test]
    fn match_blocks_are_ignored_until_the_next_host() {
        let cfg =
            "Host real\n  HostName r.example.com\n  User me\n\nMatch host nope\n  User hijack\n";
        let hosts = parse(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("me"));
    }

    #[test]
    fn invalid_port_falls_back_to_22() {
        let hosts = parse("Host p\n  HostName p.example.com\n  User me\n  Port notanumber\n");
        assert_eq!(hosts[0].port, 22);
    }

    #[test]
    fn identity_file_maps_to_public_key_auth() {
        let hosts = parse("Host k\n  HostName k.example.com\n  User me\n  IdentityFile ~/.ssh/k\n");
        let conn = hosts[0].to_connection();
        match conn.auth_method {
            AuthMethod::PublicKey { key_path } => assert_eq!(key_path, "~/.ssh/k"),
            other => panic!("expected PublicKey, got {:?}", other),
        }
    }

    #[test]
    fn no_identity_file_maps_to_agent_auth() {
        let hosts = parse("Host a\n  HostName a.example.com\n  User me\n");
        assert!(matches!(
            hosts[0].to_connection().auth_method,
            AuthMethod::Agent
        ));
    }

    /// Scratch directory unique to one test, cleaned up on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("ghost-sshcfg-{}-{}", tag, std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
        fn write(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.0.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, contents).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn include_directives_pull_in_other_files() {
        let dir = TempDir::new("include");
        dir.write(
            "extra.conf",
            "Host extra\n  HostName extra.example.com\n  User me\n",
        );
        let main = dir.write(
            "config",
            "Include extra.conf\n\nHost main\n  HostName main.example.com\n  User me\n",
        );

        let hosts = parse_file(&main).unwrap();
        let names: Vec<&str> = hosts.iter().map(|h| h.alias.as_str()).collect();
        assert!(names.contains(&"main"), "got {:?}", names);
        assert!(names.contains(&"extra"), "got {:?}", names);
    }

    #[test]
    fn include_globs_are_expanded() {
        let dir = TempDir::new("glob");
        dir.write(
            "conf.d/a.conf",
            "Host ga\n  HostName ga.example.com\n  User me\n",
        );
        dir.write(
            "conf.d/b.conf",
            "Host gb\n  HostName gb.example.com\n  User me\n",
        );
        let main = dir.write("config", "Include conf.d/*.conf\n");

        let hosts = parse_file(&main).unwrap();
        let mut names: Vec<&str> = hosts.iter().map(|h| h.alias.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["ga", "gb"]);
    }

    #[test]
    fn include_cycles_terminate() {
        // OpenSSH tolerates these; unguarded recursion here would hang.
        let dir = TempDir::new("cycle");
        dir.write(
            "b.conf",
            "Include config\nHost cyc\n  HostName c.example.com\n  User me\n",
        );
        let main = dir.write("config", "Include b.conf\n");

        let hosts = parse_file(&main).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "cyc");
    }

    #[test]
    fn a_missing_include_is_ignored_not_fatal() {
        let dir = TempDir::new("missing");
        let main = dir.write(
            "config",
            "Include does-not-exist.conf\nHost ok\n  HostName ok.example.com\n  User me\n",
        );
        let hosts = parse_file(&main).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "ok");
    }

    #[test]
    fn only_the_first_identity_file_is_kept() {
        let hosts = parse(
            "Host m\n  HostName m\n  User me\n  IdentityFile ~/.ssh/a\n  IdentityFile ~/.ssh/b\n",
        );
        assert_eq!(hosts[0].identity_file.as_deref(), Some("~/.ssh/a"));
    }
}
