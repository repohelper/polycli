# CodexCTL

Codex CLI Profile Manager - Manage multiple OpenAI Codex CLI accounts

## Installation

```bash
npm install -g codexctl
```

Or use npx (no install):
```bash
npx codexctl --help
```

## Usage

```bash
# Save your current Codex CLI profile
cdx save work
cdx save personal

# Switch between profiles
cdx load work
cdx load personal

# List all profiles
cdx list

# Quick-switch to previous profile
cdx load -

# Auto-switch to best profile based on quota
cdx load auto
```

## Features

- 🔐 **Optional Encryption** - age-based encryption for sensitive auth data
- 🚀 **Fast Switching** - Switch accounts in < 1 second
- 🤖 **Auto-Switcher** - Automatically pick the best profile based on quota
- 📊 **Real-Time Quota** - Live usage data from OpenAI API
- 🌳 **Concurrent Usage** - Use multiple profiles simultaneously

## Documentation

Full documentation: https://codexctl.repohelper.com

## License

MIT
