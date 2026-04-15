Known project release practices established so far:
- use patch releases for maintenance-only work
- keep `main` and `trunk` concerns separate when needed
- verify crates, npm, and workflow health before tagging
- prefer current compatible dependency lines and refresh `Cargo.lock` deliberately
- document branch-specific rationale when dismissing or triaging security alerts
