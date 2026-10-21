| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-url-final4 (finish pure-Rust cRZURL owner, web-network §2.1/2.7, Q4-Q6 answered) | `core-url-final4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: finish cRZURL core in engine/core/src/rzurl.rs: Q4 ChangeURL chain on N's own fields; lazy GetUser/
  GetPassword/GetParams/GetQuery/GetFragment; 8 component setters; SetFullURL(text, invalidate); ChangeURL(new, base URL)
  overload; inventory audit vs §2.1. Pure core only.
- Done: worktree created from main d91cb866.
- Next: edit rzurl.rs + tests, qbuild quick, then all, then report.
- Facts: no game run planned (core only, no plugin wiring).
