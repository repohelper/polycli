---
name: release-maintenance
description: Use when maintaining codexctl release quality, including dependency refreshes, changelog updates, cargo and npm publish readiness, workflow hygiene, and final verification gates before tagging or shipping.
---

# CodexCTL Release Maintenance

Use this skill when preparing a patch, minor, or release-quality pass.

## Workflow

1. Refresh dependencies conservatively.
2. Run the full Rust quality gates.
3. Update user-facing docs if command surface or behavior changed.
4. Update `CHANGELOG.md` with only materially user-visible maintenance or security items.
5. Keep release hygiene separate from product shaping.

## Validation stack

Run these before release work is considered done:
- `cargo fmt --all -- --check`
- `cargo test --all-features --all-targets`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release --locked`
- `cargo audit --deny warnings`

## Guardrails

- Do not cut a minor release for pure hygiene changes unless there is user-visible value.
- Do not update crypto or workflow dependencies casually; verify impact first.
- Do not merge branch-specific feature work just to fix release hygiene.

## References

Read `references/release-notes.md` when touching packaging or release automation.
