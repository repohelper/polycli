Current bounded contexts from `.codexctl/shapeup/shapeup-domain-map.md`:
- Profile Catalog
- Live Auth Projection
- Usage Intelligence
- Validation
- Task Definition
- Run Orchestration
- Run Ledger
- Release Engineering

Boundary rules to preserve:
- Validation must not know profile persistence details.
- Run Ledger stores facts, not policy.
- Run Orchestration decides policy.
- Usage Intelligence stays separate from unattended execution.
- Task Definition is intentionally opinionated and should not model arbitrary graphs.
