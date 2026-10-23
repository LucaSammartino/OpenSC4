| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animsprite5b (animated sprite settled owner behavior) | Codex / core-animsprite5b | in progress | Implement settled AS4/AS5/AS11 contracts and audit owner closure |

## Handoff
- Goal: close settled animated sprite owner behavior; keep AS1/AS2/AS3/AS6/AS7/AS10 open.
- Done: implemented AS11 foreign acquisition/output/one-release route; corrected AS5 signed-count/full endpoint behavior; transactional controller ticks; raw play-mode and slot-17 coverage.
- Done: AS4 prefix/header/factory lifetime tests and all eight conformance rows; audited remaining table boundaries to AS1/AS2/AS3/AS6/AS7/AS10.
- Validation: focused animated_sprite run 48/48 before final two tests; final owned-file fmt passed (4 files). Wrapper q.py failed to launch Bash (Win32 error 5); direct Cargo worked.
- Next: run the required qbuild all on stable final source, collect its actual terminal result, then finalize report/queue.
- No plugin/capi/ABI changes or game request planned (core foreign-service model only).
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-animsprite5b-r1-manifest.json.
