# Codex Controller (`codexctl`)

[![CI](https://github.com/repohelper/codexctl/actions/workflows/ci.yml/badge.svg)](https://github.com/repohelper/codexctl/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.94%2B-blue.svg)](https://www.rust-lang.org)

> **Codex Controller** - Full control plane for Codex CLI

**Prerequisite**: Requires [@openai/codex](https://www.npmjs.com/package/@openai/codex) to be installed first.

**Version**: 0.6.3 | **Author**: [Bhanu Korthiwada](https://github.com/BhanuKorthiwada)

🔗 **Website**: [codexctl on GitHub](https://github.com/repohelper/codexctl)  
📖 **Documentation**: See README for usage

---

## Why Codex Controller?

Codex Controller starts from a practical limitation in Codex CLI today: there is no native first-class multi-profile workflow. This repo starts with a full end-to-end control plane for profile management, usage visibility, switching, and concurrent terminal usage.

Use it when you need to:

- 🔐 **Multi-account management** - Switch between work/personal/dev accounts instantly
- 🤖 **Automation** - Run Codex in CI/CD with specific credentials
- 📊 **Usage monitoring** - Track quota across teams
- 🌳 **Concurrent sessions** - Use multiple accounts in parallel
- 🔄 **Profile-based workflows** - Environment-specific configurations

- 🔐 **Securely store** multiple Codex CLI profiles with optional encryption
- ⚡ **Switch instantly** between accounts without re-authenticating
- 🤖 **Auto-switch** based on quota availability
- 📊 **Monitor usage** across all your Codex accounts
- 🌳 **Use concurrently** - different accounts in different terminals

---

## Features

### Multi-Account Management
- 🔐 **Secure Profiles** - Store multiple Codex credentials with optional encryption
- ⚡ **Instant Switching** - Switch accounts in < 1 second
- 🔄 **Quick Toggle** - Toggle between current and previous with `codexctl load -`
- 🗂️ **Full CRUD** - Save, load, list, delete, backup profiles

### Automation & Control
- 🤖 **Auto-Switcher** - Automatically pick best profile based on quota
- 📊 **Usage Monitoring** - Real-time quota and billing data
- ✅ **Verify** - Validate all profiles' authentication status
- 🌳 **Concurrent Sessions** - Use multiple accounts in parallel
- 🏃 **CI/CD Integration** - Run with specific credentials in pipelines

### Developer Experience
- 🖥️ **Cross-Platform** - macOS, Linux, Windows (WSL2)
- 🔧 **Shell Completions** - Bash, Zsh, Fish, PowerShell
- 🧪 **Zero Runtime** - Single binary, no Node.js required
- 🐳 **Docker** - Multi-arch images
- 📦 **Import/Export** - Transfer profiles between machines

---

## Quick Start

### Prerequisites

First, install Codex CLI:

```bash
# Install Codex CLI (required)
npm install -g @openai/codex

# Verify installation
codex --version
```

### Install `codexctl`

```bash
# Via cargo
cargo install codexctl

# Or via npm
npm install -g codexctl

# Or download binary from GitHub Releases
curl -fsSL https://github.com/repohelper/codexctl/releases

# Or via Homebrew (macOS/Linux)
brew install repohelper/tap/codexctl
```

### First Steps

```bash
# Save your current Codex CLI profile
codexctl save work

# Create another profile
# (switch accounts in Codex CLI, then:)
codexctl save personal

# List all profiles
codexctl list

# Switch to a profile
codexctl load work

# Quick-switch to previous profile
codexctl load -
```

---

## Commands

```
codexctl save <name>              Save current Codex auth as a profile
codexctl load <name>              Load a saved profile and switch to it
codexctl list                     List all saved profiles
codexctl delete <name>            Delete a saved profile
codexctl status                   Show current profile status
codexctl usage                    Show usage limits and subscription info
codexctl verify                   Verify all profiles' authentication status
codexctl backup                   Create a backup of current profile
codexctl run --profile <name> -- <cmd>
                                  Run a command with a specific profile
codexctl env <name>               Export shell commands for concurrent usage
codexctl diff <name1> <name2>     Compare/diff two profiles
codexctl switch                   Switch to a profile interactively (fzf)
codexctl history                  View command history
codexctl doctor                   Run health check on profiles
codexctl completions              Generate shell completions
codexctl import <name> <b64>      Import a profile from another machine
codexctl export <name>            Export a profile for transfer
codexctl setup                    Interactive setup wizard
```

---

## Encryption

```bash
# Save with encryption
codexctl save work --passphrase "my-secret"

# Or use environment variable
export CODEXCTL_PASSPHRASE="my-secret"
codexctl save work

# Load encrypted profile
codexctl load work --passphrase "my-secret"
```

---

## Usage And Auto-Switching

Inspect usage directly or let the controller pick the best available profile:

```bash
# Show current profile usage details
codexctl usage

# Compare usage across all saved profiles
codexctl usage --all

# Switch to the profile with the best remaining quota
codexctl load auto
```

---

## Concurrent Usage

Run different Codex identities in separate terminals:

```bash
# Print shell exports for a profile
codexctl env work

# Bash/Zsh example
eval "$(codexctl env work)"

# Run one command against a specific profile and restore after
codexctl run --profile work -- codex --version
```

---

## Shell Completions

```bash
source <(codexctl completions bash)
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

## Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](./LICENSE) for details.

---

Built by [Bhanu Korthiwada](https://github.com/BhanuKorthiwada)
