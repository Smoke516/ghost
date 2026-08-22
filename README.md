# 👻 Ghost

A terminal SSH connection manager: keep your hosts in one list, see which are
reachable, and open a session in one keystroke.

![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)

## What it does

- **One list of hosts**, imported straight from your `~/.ssh/config` or added by hand.
- **Background reachability checks** so you can see what's up before you try it.
- **One-keystroke connect** — `Enter`, or `1`–`9` for the first nine hosts.
- **Opens in a new terminal window** when your emulator supports it, otherwise
  hands the current terminal over to `ssh`.
- **Search** across name, host, user, tag, and description.
- **Twelve themes**, three panel layouts.

Ghost does not replace `ssh`. It builds an `ssh` command line and runs it, so
your `~/.ssh/config`, agent, and known_hosts all behave exactly as usual.

## Install

### From source

```bash
git clone https://github.com/Smoke516/ghost.git
cd ghost
cargo install --path .
```

### From crates.io

```bash
cargo install ghost-ssh   # the binary is named `ghost`
```

### Prebuilt binaries

Grab one from [Releases](https://github.com/Smoke516/ghost/releases). Builds are
published for Linux (x64, arm64), macOS (Intel, Apple Silicon), and Windows x64.

## Getting started

```bash
# Import everything from ~/.ssh/config
ghost --import-ssh-config --dry-run   # see what would be imported
ghost --import-ssh-config             # actually import

# Launch the TUI
ghost
```

Inside the TUI, `i` runs the same import, `a` adds a host by hand, and `h` shows
every keybinding.

## Keys

| Key | Action |
| --- | --- |
| `j` `k` / `↑` `↓` | Move through the list |
| `g` `G` / `PgUp` `PgDn` | Jump to ends / move by ten |
| `Enter` | Connect to the selected host |
| `1`–`9` | Quick connect by position |
| `/` | Search (name, host, user, tag, description) |
| `a` / `e` / `d` | Add / edit / delete a host |
| `i` | Import from `~/.ssh/config` |
| `r` | Re-check reachability of every host |
| `f` | Show only reachable hosts |
| `S` `A` `H` | Sessions / analytics / history |
| `l` / `[` `]` | Cycle layout / resize panels |
| `T` / `t` | Next theme / theme picker |
| `Ctrl+X` | Terminate all tracked sessions |
| `h` or `F1` | Help |
| `q` / `Ctrl+C` | Quit |

`Esc` clears an active search; press it again to quit.

## Command line

```
ghost                            Launch the TUI
ghost --new-terminal             Always open a new terminal window
ghost --direct                   Always connect in the current terminal
ghost --connection-mode MODE     auto | new-terminal | direct
ghost --import-ssh-config [PATH] Import hosts and exit (default: ~/.ssh/config)
ghost --import-ssh-config --dry-run    Show what would be imported
```

### Connection modes

- **auto** (default) — open a new terminal window if a supported emulator is
  found, otherwise fall back to direct.
- **new-terminal** — always open a window; error out if none is available.
- **direct** — tear down the TUI and hand this terminal to `ssh`, restoring the
  TUI when the session ends. Use this inside multiplexers, or in terminals that
  can't be told to launch a command (Warp).

Supported for new windows: Ghostty, Alacritty, Kitty, WezTerm, GNOME Terminal,
Konsole, XFCE Terminal, xterm, Windows Terminal, macOS Terminal.

> **Session tracking caveat.** GNOME Terminal and Konsole are client/server: the
> process Ghost spawns hands off to a daemon and exits immediately, so there is
> no PID worth tracking. Ghost detects this and skips session tracking for them
> rather than showing a session that instantly disappears.

## Configuration

Config lives at `~/.config/ghost/config.toml` (`%APPDATA%\ghost\config.toml` on
Windows) and is written with `0600` permissions. Ghost rewrites it whenever you
add, edit, or delete a host in the TUI; writes are atomic, so an interrupted
save can't truncate your host list.

Every `[settings]` key is optional. See
[`example-config.toml`](example-config.toml) for a documented sample.

```toml
[settings]
theme = "TokyoNightDark"
refresh_interval = 30       # seconds between reachability checks
show_only_online = false

[servers.web]
name = "Web"
host = "web.example.com"
port = 2222
username = "deploy"
description = "Frontend"
tags = ["production"]
timeout = 10                # ssh ConnectTimeout, optional

[servers.web.auth_method]
type = "public_key"         # agent | public_key | password | interactive
key_path = "~/.ssh/id_ed25519"
```

## Status column

```
● online    ● offline    ◐ checking    ? not yet checked
🔑 key or agent auth    ⚠ password auth    💬 keyboard-interactive
```

The auth icon reflects **your local connection config**, not the remote host's
security posture. Ghost's reachability check is a TCP connect; it cannot and
does not audit whether a server is patched or hardened. The icon exists so that
password auth stands out against key auth in a long list.

`Uptime` in the details panel is the share of Ghost's own reachability probes
that succeeded — it is not the remote host's uptime.

## Security notes

- Ghost never stores passwords or key material. Public-key entries store a
  *path*; authentication is performed entirely by `ssh`.
- The `ssh` command line is assembled as discrete argv elements and executed
  without a shell, so a hostname or username containing shell metacharacters
  cannot be interpreted as a command. This is covered by tests.
- The config file is written `0600` via an atomic
  write-to-temp-then-rename.

## Building and testing

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Minimum supported Rust version: 1.85.

## Troubleshooting

**Nothing happens when I press Enter.** Your terminal emulator may not be
detected. Run `ghost --direct` to connect in the current terminal instead.

**Everything shows as offline.** Ghost's check is a plain TCP connect to
`host:port`. A host behind a bastion, using port knocking, or dropping
unsolicited connections will read as offline but still connect fine — press
Enter and let `ssh` be the judge.

**Ghostty prints OSC warnings.** Add `log-level = error` to
`~/.config/ghostty/config`.

**Windows Defender flags the binary.** A common false positive for unsigned Rust
binaries. Verify against the release checksums, or build from source.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
