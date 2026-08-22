# Changelog

All notable changes to Ghost are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Import from `~/.ssh/config`** — `i` in the TUI, or `ghost --import-ssh-config`
  from the shell (with `--dry-run` to preview). Understands `Host`, `HostName`,
  `User`, `Port`, `IdentityFile`, and `Include` (including globs and cycles).
  Wildcard blocks like `Host *` are not imported.
- **Search** — `/` filters the list live across name, host, username, tags, and
  description. `Esc` clears it.
- **Theme picker** — `t` opens a real theme list with live preview; `Enter`
  keeps, `Esc` restores. Previously `t` toggled a flag nothing rendered.
- **Per-server connect timeout** is now editable in the form and passed to `ssh`
  as `ConnectTimeout`.
- Navigation: `g`/`G` jump to the ends, `PgUp`/`PgDn` move ten at a time.
- Empty-state panel that distinguishes "no servers configured" from "your filter
  matched nothing" and offers the relevant next action.
- 59 tests, up from 8.

### Fixed

- **Typing any non-ASCII character in a form crashed Ghost.** The input cursor
  was a byte index advanced one per character, so the keystroke after `é` (or
  any emoji or CJK character) landed mid-codepoint and `String::insert` aborted
  the process, leaving the terminal in raw mode.
- **A panic no longer wrecks your terminal.** A panic hook restores cooked mode
  and leaves the alternate screen before the backtrace prints.
- **Offline/online notifications never fired.** The new status was written
  before being compared against the old one, so every transition check was
  false.
- **Analytics could panic.** The success rate computed
  `total_connections - total_failures` on independent `u32` counters, which
  underflowed as soon as failures exceeded connections.
- **Servers added after startup were never health-checked.** The monitor
  captured a snapshot of the server list at launch; it now re-reads the list
  every cycle. If the config started empty, monitoring never started at all.
- **The server list did not scroll.** It rendered as a stateless widget, so
  selecting past the visible rows highlighted an off-screen entry.
- **`r` froze the UI.** Health checks ran sequentially inside the key handler,
  up to five seconds each. They now run concurrently in the background; the UI
  keeps drawing and `Esc` works.
- **Saving wiped configured fields.** `timeout` was hardcoded to `None` on every
  save, and `created_at` was never persisted, so creation dates reset to "today"
  on each launch.
- **`kill -0` was forked ~20×/second per tracked session** to test liveness.
  Liveness is now a direct syscall, throttled to once every two seconds.
- **PID 0 is rejected.** On POSIX, `kill(0, sig)` signals the caller's entire
  process group — Ghost included.
- The latency sparkline rendered backwards (newest-first) and showed a fake
  flat series when no samples existed.
- The form's Tab order could not reach the auth-method selector going forward.
- The form overflowed an 80×24 terminal, silently dropping fields; it now
  scrolls to keep the focused row visible.
- The help screen was truncated and its indentation stripped; it now scrolls.
- A config missing any `[settings]` key failed to parse; all settings now
  default individually.
- `example-config.toml` did not parse (`theme = "tokyo-night"` is not a valid
  variant). A test now keeps it valid.

### Changed

- **All twelve themes now actually apply.** The UI carried 223 hardcoded Tokyo
  Night constants; switching themes changed almost nothing on screen.
- **First run no longer invents three fictional servers** (`prod.example.com`
  and friends) that were health-checked and then written into your config.
- **Connection stats are no longer conflated.** Reachability probes update their
  own counters; `connection_count` now means "sessions you opened". Uptime is
  the share of successful probes, and is labelled as such.
- Session tracking is skipped for GNOME Terminal and Konsole, which hand off to
  a daemon and exit — the captured PID was dead on arrival.
- `refresh_interval` and `panel_layout` from the config are now honoured. The
  `animation_speed` and `smooth_animations` keys were never read and have been
  removed (unknown keys are ignored, so old configs still load).
- Upgraded to ratatui 0.29 / crossterm 0.28. MSRV is now 1.88, the highest floor in the locked dependency graph.
- Dropped `russh`, `russh-keys`, `serde_json`, `async-trait`, and `config` —
  none were referenced anywhere. `tokio`'s `full` feature was narrowed to the
  five features actually used.
- CI now passes: `cargo fmt --check` and `cargo clippy -- -D warnings` were both
  failing on `main`. Replaced the archived `actions-rs/toolchain` and the
  shut-down `upload-artifact@v3`, and added aarch64 Linux and Apple Silicon to
  the build matrix.
- `Cargo.lock` is committed, as it should be for a binary crate.
- README rewritten to describe what the code does. It had advertised
  "🛡️ SECURE / ⚠️ VULNERABLE" indicators and a `servers.json` file that do not
  exist, and `cargo install ghost` (the crate is `ghost-ssh`).

### Security

- The auth indicator is now documented as reflecting **local configuration
  only**. It was presented as a security assessment of the remote host, which it
  never was and cannot be from a TCP connect.
