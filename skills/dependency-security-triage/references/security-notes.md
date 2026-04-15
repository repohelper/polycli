Repository lessons already established:
- GitHub Dependabot alerts are repo/default-branch scoped, not branch-local objects.
- A real fix may be impossible if the vulnerable crate is only reachable through a stable upstream dependency line.
- In this repo, `GHSA-cq8v-f236-94qc` was traced to `age 0.11.2 -> rand 0.8.5`, but the required `rand` `log` feature path was not enabled on either `main` or `trunk`.
- For this project, changing the crypto stack just to silence a low-severity non-reachable advisory is usually the wrong tradeoff.
