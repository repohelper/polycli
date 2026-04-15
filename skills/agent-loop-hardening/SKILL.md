---
name: agent-loop-hardening
description: Use when modifying codexctl unattended execution, including validate, run-loop, runs, resume behavior, repo-state policy, notification hooks, run ledgers, and deterministic acceptance or review gates. This skill is for hardening the loop kernel without broadening it into a general orchestrator.
---

# CodexCTL Run Loop Hardening

Use this skill for runtime execution work around `validate`, `run-loop`, and `runs`.

## Goals

- preserve deterministic validation as the trust boundary
- keep run state durable and inspectable
- protect unattended execution from hanging hooks and unsafe repo states
- avoid expanding the loop into CAR-style orchestration

## Workflow

1. Read `references/runtime-contracts.md` before changing command behavior.
2. Trace the change across:
   - CLI flags in `src/main.rs`
   - command implementation in `src/commands/`
   - persisted types in `src/utils/runs.rs` or `src/utils/task.rs`
   - integration tests in `tests/`
3. Prefer explicit stop reasons, stable JSON output, and stable exit codes.
4. If adding policy, record it in human output, JSON output, and tests.
5. For repo-state changes, keep `.codexctl/` as project-owned planning space.

## Guardrails

- Do not add queueing or multi-agent orchestration here.
- Do not let notify hooks change the primary run outcome.
- Do not mark success unless deterministic checks pass.
- Do not break resume safety to gain convenience.

## Validation

After changes, run:
- `cargo test --all-features --all-targets`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release --locked`
