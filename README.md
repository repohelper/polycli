# CodexCTL

[![CI](https://github.com/repohelper/codexctl/actions/workflows/ci.yml/badge.svg)](https://github.com/repohelper/codexctl/actions)
[![npm](https://img.shields.io/npm/v/codexctl.svg)](https://www.npmjs.com/package/codexctl)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.94%2B-blue.svg)](https://www.rust-lang.org)

> **Codex CLI Profile Manager** - Manage multiple OpenAI Codex CLI accounts

**Version**: 0.2.0 | **Author**: [Bhanu Korthiwada](https://github.com/BhanuKorthiwada) | **Status**: ✅ Public Beta

🔗 **Website**: [codexctl.repohelper.com](https://codexctl.repohelper.com)  
📖 **Documentation**: [codexctl.repohelper.com/docs](https://codexctl.repohelper.com/docs)

---

## Why CodexCTL?

If you work with multiple OpenAI Codex CLI accounts (work, personal, side projects), CodexCTL lets you:

- 🔐 **Securely store** multiple Codex CLI profiles with optional encryption
- ⚡ **Switch instantly** between accounts without re-authenticating
- 🤖 **Auto-switch** based on quota availability
- 📊 **Monitor usage** across all your Codex accounts
- 🌳 **Use concurrently** - different accounts in different terminals

---

## Features

### Core
- 🔐 **Optional Encryption** - age-based encryption for sensitive auth data
- 🚀 **Fast Switching** - Switch accounts in < 1 second
- 🔄 **Quick-Switch** - Toggle between current and previous profile with `cdx -`
- 🗂️ **Profile Management** - Save, load, list, delete, backup profiles

### Advanced
- 🤖 **Auto-Switcher** - Automatically pick the best profile based on quota availability
- 📊 **Real-Time Quota** - Live usage data from OpenAI API
- ✅ **Verify Command** - Validate all profiles' authentication status
- 🌳 **Concurrent Usage** - Use multiple profiles simultaneously via `env` command
- 📦 **Import/Export** - Transfer profiles between machines securely

### Developer Experience
- 🖥️ **Cross-Platform** - macOS, Linux, Windows support
- 🔧 **Shell Completions** - Bash, Zsh, Fish, PowerShell
- 🧪 **Zero Dependencies** - Single binary, no runtime requirements
- 🐳 **Docker Support** - Multi-arch images available
- 🧬 **Auto-Migration** - Seamless upgrades between versions

---

## Quick Start

### Install

```bash
# Via cargo
cargo install codexctl

# Or via npm
npm install -g codexctl

# Or download binary
curl -fsSL https://codexctl.repohelper.com/install.sh | sh

# Or via Homebrew (macOS/Linux)
brew install repohelper/tap/codexctl
```

### First Steps

```bash
# Save your current Codex CLI profile
cdx save work

# Create another profile
# (switch accounts in Codex CLI, then:)
cdx save personal

# List all profiles
cdx list

# Switch to a profile
cdx load work

# Quick-switch to previous profile
cdx load -
```

---

## Commands

```
cdx save <name>              Save current Codex auth as a profile
cdx load <name>              Load a saved profile and switch to it
cdx list                     List all saved profiles
cdx delete <name>            Delete a saved profile
cdx status                   Show current profile status
cdx usage                    Show usage limits and subscription info
cdx verify                   Verify all profiles' authentication status
cdx backup                   Create a backup of current profile
cdx run <name> -- <cmd>      Run a command with a specific profile
cdx env <name>               Export shell commands for concurrent usage
cdx diff <name1> <name2>     Compare/diff two profiles
cdx switch                   Switch to a profile interactively (fzf)
cdx history                  View command history
cdx doctor                   Run health check on profiles
cdx completions              Generate shell completions
cdx import <file>            Import a profile from another machine
cdx export <name>            Export a profile for transfer
cdx setup                    Interactive setup wizard
```

---

## Encryption (Optional)

```bash
# Save with encryption
cdx save work --passphrase "my-secret"

# Or use environment variable
export CODEXCTL_PASSPHRASE="my-secret"
cdx save work

# Load encrypted profile
cdx load work --passphrase "my-secret"
```

---

## Auto-Switcher

Let CodexCTL pick the best profile automatically:

```bash
# Switch to profile with most quota available
cdx load auto

# Configure auto-switch preferences
cdx config set auto_switch.threshold 80
cdx config set auto_switch.prefer work,personal
```

---

## Shell Integration

Add to your `.bashrc`/`.zshrc`:

```bash
# Enable completions
source <(cdx completions bash)

# Optional: Auto-switch based on directory (like direnv)
eval "$(cdx init --shell zsh)"
```

---

## Docker

```bash
# Run with Docker
docker run -it --rm \
  -v ~/.codexctl:/root/.config/codexctl \
  -v ~/.codex:/root/.codex \
  ghcr.io/repohelper/codexctl list
```

---

## Configuration

Configuration directory: `~/.config/codexctl/`

```toml
# ~/.config/codexctl/config.toml
[default]
cli = "codex"  # Default CLI to manage

[auto_switch]
enabled = true
threshold = 80  # Switch when quota below 80%

[encryption]
default_passphrase = false  # Always prompt for passphrase
```

---

## Comparison

| Feature | CodexCTL | [codex-profiles](https://github.com/midhunmonachan/codex-profiles) |
|---------|---------|-------------------------------------------------------------------|
| Encryption | ✅ | ❌ |
| Auto-Switcher | ✅ | ❌ |
| Real-Time Quota | ✅ | ❌ |
| Shell Completions | ✅ | ❌ |
| Docker Support | ✅ | ❌ |
| Cross-Platform | ✅ | ✅ |

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](./LICENSE) for details.

---

**Made with ❤️ by [Bhanu Korthiwada](https://github.com/BhanuKorthiwada)**  
Part of the [RepoHelper](https://repohelper.com) project collection.
