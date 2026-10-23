| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-app5b (application owner AP12–AP15) | Codex / core-app5b (detached HEAD) | in progress | Implement settled owner contracts and conformance vectors; full gate |

## Handoff
- Goal: close application owner rows settled by AP12–AP15, preserving AP3–AP11 boundaries.
- Done: b5442908 initial handoff. Implemented AP12 plugin policy, AP13/AP14 paths, AP15 directory dispatch, AP14 folded getters/video/preference algorithms and focused tests.
- Focused evidence: build-runs/focused-second.log, 54 application tests passed (before final formatting and signed-language getter wrapper). First compile failed only on test Preferences::default; corrected to existing fixture. q wrapper Git Bash failed before cargo; build-runs/focused.py logs native Cargo.
- Next: implement resources/fonts/control/cursor tables, save-path naming and obliteration failure behavior; integrate settled methods into owned runner, complete closure table and conformance tests; run explicit fmt then stable all gate.
- Formatting: build-runs/format-patch.py generates stdin-rustfmt diff, applied only through apply_patch; explicit qbuild fmt still pending.
- Preference fullscreen bool retains its semantic meaning; raw +F17 is windowed and now serializes inversely. CheatRequests::step exposes native nonprogress without an infinite blocking call.
- No builds or game requests outstanding; no ABI/plugin/capi changes permitted.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-app5b-r1-manifest.json.
