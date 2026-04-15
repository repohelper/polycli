Relevant local sources of truth:
- `.codexctl/shapeup/contracts/shapeup-contract-bet-01.md`
- `.codexctl/shapeup/bets/shapeup-bet-01-loop-kernel-foundations.md`

Current product stance learned from implementation:
- `.codexctl/tasks/` is the repo-local control plane for shaped work
- `run-loop` may start with dirty files under `.codexctl/` only
- notify hooks must not stall completion indefinitely
- review checks are distinct from acceptance checks
- JSON mode should emit one final object only
