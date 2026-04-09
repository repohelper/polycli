# Codexo

[![CI](https://github.com/BhanuKorthiwada/codexo/actions/workflows/ci.yml/badge.svg)](https://github.com/BhanuKorthiwada/codexo/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)

> The ultimate CLI for managing multiple OpenAI Codex accounts

Codexo lets you seamlessly switch between multiple Codex accounts without repeated `codex login`. Features optional encryption, auto-switching, real-time quota monitoring, and much more.

## Features

- 🔐 **Optional Encryption** - Secure your auth tokens with age-based encryption
- 🚀 **Fast Switching** - Switch accounts in < 1 second
- 🤖 **Auto-Switcher** - Automatically pick the best profile based on quota
- ⚡ **Quick-Switch** - Toggle between current and previous profile with `codexo load -`
- 📊 **Real-Time Quota** - Live usage data from OpenAI API
- 🔄 **Auto-Backup** - Automatic backups before switching
- 🌳 **Concurrent Profiles** - Use multiple profiles simultaneously
- 🖥️ **Cross-Platform** - macOS, Linux, Windows support
- 🧪 **Well Tested** - Comprehensive test suite

## Quick Start

### Installation

```bash
# Via cargo
cargo install codexo

# Via npm (coming soon)
npm install -g codexo

# Or download binary from releases
curl -fsSL https://github.com/BhanuKorthiwada/codexo/releases/latest/download/codexo-$(uname -m)-unknown-linux-gnu.tar.gz | tar xz
```

### Usage

```bash
# Save current Codex auth as "work" profile
codexo save work

# Switch to work account
codexo load work

# Toggle between current and previous
codexo load -

# Auto-switch to best available profile
codexo load auto

# List all profiles
codexo list

# Verify all profiles are valid
codexo verify

# View usage across all profiles
codexo usage --all

# Save with encryption
codexo save work --passphrase "my secret"
```

## Commands

| Command | Description | Example |
|---------|-------------|---------|
| `save` | Save current auth as profile | `codexo save work --passphrase "secret"` |
| `load` | Switch to a profile | `codexo load work` or `codexo load -` or `codexo load auto` |
| `list` | List all profiles | `codexo list --detailed` |
| `delete` | Delete a profile | `codexo delete old-work --force` |
| `status` | Show current profile | `codexo status` |
| `usage` | Show subscription info | `codexo usage --all --realtime` |
| `verify` | Validate all profiles | `codexo verify` |
| `backup` | Create backup | `codexo backup` |
| `run` | Run with profile | `codexo run work -- codex` |
| `env` | Export env vars | `eval $(codexo env work)` |
| `diff` | Compare profiles | `codexo diff work personal` |
| `switch` | Interactive selector | `codexo switch` |
| `history` | View history | `codexo history --limit 20` |
| `doctor` | Health check | `codexo doctor` |
| `completions` | Shell completions | `codexo completions bash --install` |

## Security

- **Path Traversal Protection**: Profile names are validated to prevent directory traversal
- **Optional Encryption**: Uses [age](https://github.com/FiloSottile/age) encryption for auth tokens
- **Atomic Operations**: Profile switches are atomic - no partial states
- **No External Dependencies**: Single binary, no runtime requirements

## License

MIT © Bhanu Korthiwada

## Contributing

Contributions welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

---

Built with Rust 🦀
