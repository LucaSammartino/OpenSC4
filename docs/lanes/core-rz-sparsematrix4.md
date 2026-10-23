| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rz-sparsematrix4: first portable sparse matrix | Codex; core-rz-sparsematrix4 | in progress | Read contract; baseline; implement bounded core algorithms |

## Handoff
- Goal: first standalone Rust sparse matrix/column iterator, explicit validated fixture state only.
- Done: reused reserved detached worktree; clean initial status; read operating rules and contract outline.
- Next: baseline qbuild all, finish contract sections and SM1-SM5, implement and test.
- No native constructor, lifetime, resize, factory, plugin or ABI implementation authorized.
- No game requests outstanding; standalone core has no game wiring.
