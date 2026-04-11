# CodexCTL

Codex CLI Profile Manager - Manage multiple AI CLI accounts (Codex, Claude, Gemini, OpenAI)

## Installation

```bash
npm install -g codexctl
```

Or use npx (no install):
```bash
npx codexctl --help
```

## Quick Start

```bash
# Save your current CLI profile
codexctl save work
codexctl save personal

# Switch between profiles
codexctl load work

# List all profiles
codexctl list
```

## Features

- Optional encryption for sensitive auth data
- Fast profile switching
- Multiple AI CLI support (Codex, Claude, Gemini, OpenAI)
- Export to use profiles concurrently in different terminals

## Binary Package

This npm package downloads pre-built binaries from GitHub Releases on install.

Supported platforms:
- Linux (x86_64, arm64)
- macOS (x86_64, arm64)  
- Windows (x86_64)

## License

MIT