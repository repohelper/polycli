---
name: shapeup-bet-authoring
description: Use when shaping, authoring, or tightening codexctl bet specs, Shape Up planning artifacts, or DDD-aligned task definitions under .codexctl/. This skill is for turning feature ideas into strict shaped bets, improving bet quality, and keeping planning artifacts aligned with the project's Shape Up + DDD model.
---

# CodexCTL Shape Up Bet Authoring

Use this skill when the work is about shaping or improving the planning model, not implementing runtime code.

## Goals

- keep planning artifacts under `.codexctl/`
- use `shapeup` for planning/process language
- use `bet` for executable units
- keep specs narrow, opinionated, and aligned with DDD bounded contexts
- optimize for high-agency product builders using AI agents

## Workflow

1. Read the current planning source of truth before editing anything:
   - `references/shapeup-planning.md`
2. If the request changes product direction, target users, or scope boundaries, update the relevant `.codexctl/shapeup/` artifacts first.
3. Keep appetite fixed and cut scope to fit.
4. Require explicit:
   - objective
   - appetite
   - bounded contexts
   - success signal
   - no-gos
5. Prefer shaped bets over backlog-style task lists.
6. If implementation contracts change, treat that as reshaping, not a casual doc edit.

## Guardrails

- Do not broaden `.codexctl/tasks/` into a general workflow DSL.
- Do not collapse `shapeup`, `bet`, and `run` into interchangeable terms.
- Do not add queueing, orchestration, or worktree scope unless the current bet explicitly includes it.
- Do not let planning artifacts drift away from the actual command surface.

## When To Read References

- Always read `references/shapeup-planning.md` before changing planning docs or bet schemas.
- Read `references/domain-map.md` when adding or changing bounded contexts.

## Deliverables

Typical outputs:
- updated `.codexctl/shapeup/*.md` artifacts
- tighter bet specs under `.codexctl/tasks/`
- clearer cut lines, no-gos, and acceptance matrices
