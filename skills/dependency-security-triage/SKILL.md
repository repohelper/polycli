---
name: dependency-security-triage
description: Use when investigating GitHub Dependabot alerts, RustSec findings, or dependency security questions in codexctl. This skill is for verifying exploitability, comparing branch build graphs, deciding between upgrade, mitigation, or dismissal, and documenting evidence clearly.
---

# CodexCTL Security Triage

Use this skill for dependency-alert triage, not generic feature work.

## Workflow

1. Get the exact advisory details first.
2. Verify affected branches separately if the user cares about branch isolation.
3. Trace the vulnerable package with `cargo tree -i`.
4. Check whether the vulnerable path is actually enabled in this repo's feature graph.
5. Prefer technical removal or upgrade when safe.
6. If the alert is not reachable, document a precise dismissal reason with evidence.

## Decision order

1. upgrade the upstream dependency if safe and available
2. remove the triggering feature path if possible
3. patch or replace only if the risk tradeoff is justified
4. dismiss with evidence if the advisory conditions are not met

## Guardrails

- Do not dismiss blindly.
- Do not rewrite crypto stacks casually to make a scanner green.
- Do not claim a fix if the vulnerable crate is still present but only non-reachable.
- Separate repository-level GitHub alert state from branch-specific code state.

## References

Read `references/security-notes.md` when triaging crypto or transitive dependency alerts.
