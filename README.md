# 👻 Ghost SSH Manager

A modern, cross-platform SSH connection manager with a beautiful terminal UI, security assessment, and multi-terminal support.

![GitHub Release](https://img.shields.io/github/v/release/smoke516/ghost?style=flat-square)
![Platform Support](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)

## ✨ Features

### 🚀 **Connection Management**
- **Multi-platform support**: Linux, macOS, and Windows
- **Multiple connection modes**: Auto, new terminal window, or direct connection
- **Security assessment**: Automatic evaluation of SSH connection security
- **Quick connect**: Number keys (1-9) for instant server connections
- **Connection history**: Track and review your SSH activity

### 🖥️ **Terminal Integration**
- **Smart terminal detection**: Supports 10+ popular terminal emulators
- **Ghostty**, **Alacritty**, **Kitty**, **Wezterm**, **GNOME Terminal**, **Konsole**, **XFCE Terminal**, **XTerm**, **Windows Terminal**, and more
- **Warp Terminal compatible**: Optimized for modern terminal experiences
- **Fallback handling**: Graceful degradation when terminals aren't available

### 🎨 **Beautiful Interface**
- **Modern TUI**: Clean, responsive terminal user interface
- **Multiple themes**: Customizable color schemes
- **Flexible layouts**: Single, two-panel, or three-panel views
- **Real-time status**: Live connection health and session monitoring
- **Contextual help**: Interactive tooltips and comprehensive help system

### 📊 **Analytics & Monitoring**
- **Session tracking**: Monitor active SSH connections with PIDs and duration
- **Performance metrics**: Latency monitoring and connection statistics
- **Usage analytics**: Server usage patterns and connection insights
- **Health monitoring**: Background server availability checks

### 🔒 **Security Features**
- **Security assessment**: 🛡️ SECURE, ⚠️ VULNERABLE, ❓ UNKNOWN status indicators
- **Multiple auth methods**: SSH keys, SSH agent, password, and interactive authentication
- **Port security evaluation**: Non-standard ports marked as more secure
- **Connection encryption**: All connections use standard SSH protocols

## 📦 Installation

### Quick Install (Recommended)

#### Linux & macOS
```bash
curl -sSL https://raw.githubusercontent.com/smoke516/ghost/main/install.sh | bash
```

#### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/smoke516/ghost/main/install.ps1 | iex
```

### Manual Installation

#### Using Cargo (All Platforms)
```bash
cargo install ghost
```

#### From Source
```bash
git clone https://github.com/smoke516/ghost.git
cd ghost
cargo install --path .
```

#### Binary Releases
Download pre-compiled binaries from the [Releases](https://github.com/smoke516/ghost/releases) page:
- **Linux**: `ghost-linux-x64.tar.gz`
- **macOS**: `ghost-macos-x64.tar.gz` 
- **Windows**: `ghost-windows-x64.zip`

## 🚀 Quick Start

### Launch Ghost
```bash
# Auto connection mode (default)
ghost

# Force new terminal windows
ghost --new-terminal

# Direct connection mode (current terminal)
ghost --direct
```

### Basic Usage
1. **Add a server**: Press `a` to add your first SSH server
2. **Connect**: Press `Enter` or number keys (1-9) for quick connect
3. **Manage**: Use `e` to edit, `d` to delete servers
4. **Monitor**: Press `S` to view active sessions, `A` for analytics

### Connection Modes
- **Auto Mode**: Tries new terminal window, falls back to direct
- **New Terminal**: Forces new window (fails if no terminal available)  
- **Direct Mode**: Uses current terminal (Warp Terminal compatible)

## 🎮 Controls

### Navigation
- `j/k` or `↑/↓` - Navigate server list
- `Enter` or `1-9` - Connect to server
- `Esc` or `q` - Quit application

### Server Management  
- `a` - Add new server
- `e` - Edit selected server
- `d` - Delete selected server
- `r` - Refresh server status & security assessment

### Views & Features
- `S` - Session manager (active SSH sessions)
- `A` - Analytics dashboard (usage statistics)  
- `H` - Connection history
- `f` - Toggle online-only filter
- `t/T` - Theme controls
- `l` - Layout options
- `?` - Contextual help
- `h` - Full help menu

### Session Management
- `Ctrl+X` - Kill all active SSH sessions

## 🔧 Configuration

Ghost stores configuration in:
- **Linux/macOS**: `~/.config/ghost/`
- **Windows**: `%APPDATA%/ghost/`

### Files
- `config.toml` - Application settings and preferences
- `servers.json` - Server definitions and connection details

### Example Server Configuration
```json
{
  "servers": {
    "server-id": {
      "name": "My Server",
      "host": "example.com", 
      "port": 2222,
      "username": "user",
      "auth_method": "PublicKey",
      "key_path": "~/.ssh/id_rsa",
      "description": "Production server",
      "tags": ["production", "web"]
    }
  }
}
```

## 🔒 Security

Ghost prioritizes security in SSH connections:

### Security Assessment
- 🛡️ **SECURE**: SSH keys, SSH agent, non-standard ports
- ⚠️ **VULNERABLE**: Password auth on standard port 22
- ❓ **UNKNOWN**: Assessment pending or interactive auth

### Best Practices
- Use SSH key authentication when possible
- Change default SSH port (22) to non-standard ports
- Enable SSH agent for key management
- Monitor connection attempts in the analytics dashboard

## 🌍 Cross-Platform Support

### Linux
- **Supported Distros**: Ubuntu, Debian, Fedora, Arch, Pop!_OS, and more
- **Terminal Support**: GNOME Terminal, Konsole, XFCE Terminal, XTerm
- **Modern Terminals**: Alacritty, Kitty, Wezterm, Ghostty

### macOS  
- **Versions**: macOS 10.15+ (Catalina and newer)
- **Terminal Support**: Terminal.app, iTerm2, Alacritty, Kitty, Wezterm
- **Apple Silicon**: Native support for M1/M2 Macs

### Windows
- **Versions**: Windows 10/11
- **Terminal Support**: Windows Terminal, Command Prompt, PowerShell
- **Modern Options**: Alacritty, Wezterm support

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup
```bash
git clone https://github.com/yourusername/ghost.git
cd ghost
cargo build
cargo run
```

### Running Tests
```bash
cargo test
cargo clippy
cargo fmt
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for the terminal UI
- Uses [Crossterm](https://github.com/crossterm-rs/crossterm) for cross-platform terminal handling
- Inspired by modern SSH management tools and terminal applications

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/ghost/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/ghost/discussions)
- **Documentation**: [Wiki](https://github.com/yourusername/ghost/wiki)

---

**Made with ❤️ for the terminal-loving community**
