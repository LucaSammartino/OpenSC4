| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-weather-propdev6: weather and prop developer shadows | Codex / `glue-weather-propdev6` (engine, detached HEAD) | in progress | Measure weather alone, then both shadows through 2032 |

## Handoff
- Goal: default-off record-only weather and prop developer shadows against main; weather alone first.
- Reserved worktree reused at detached HEAD 5b14b8109; initial tree clean.
- Prior lane file absent from main; read from commit cea30b3cc without switching refs.
- Prior weather implementation and measurements exist on unlanded commits; WE1 daily wind arithmetic question was filed there, not on current main.
- Weather source restored from prior lane's engine-only changes and integrated additively with current main; dated monthly lines use `weather shadow <date>:`. Default-off, full save record comparison and independent CRT replay; no model resynchronization.
- Focused weather Rust tests 3/3 passed; owned weather.rs fmt passed.
- Native verify qbuild-20261024-022343-build-ctest terminal PASS: 233/233 build edges (96.4s), CTest 72/72 (162.2s), total 262.3s. Reports tree changed (source was committed while queued); final stable-source full gate remains required.
- Diagnostic: q.py cannot start Git Bash in this sandbox (CreateFileMapping error); use direct PowerShell commands. fmt paths are relative to engine/, so use capi/src/weather.rs.
- Weather-only run 374 gwp6-weather-2032-a terminal clean through 2032-01-05, ref_warnings=0. Exact count `gwp6-weather-2032-a 1/73`. WE1 filed: first step 3 wind x differs one ULP, byte 65833 118/117; final cumulative 1622 differences. Arithmetic deliberately unchanged pending contract.
- Prop developer now snapshots member and wealth MT state at each native operation boundary (PV14), copies into shadow-owned state, compares night order, never resynchronizes after mismatch, marks unsupported selector/shuffle inputs unavailable. PV15 borrowed manager path now records availability rather than treating missing input as exact daytime.
- Focused new observed-wealth test passed (copy isolation, invalid snapshot preserves continuation). Earlier compile diagnostic retained in build-runs/diagnostic-propdev-rng-a.txt. fmt on core mod recursively detects one pre-existing unowned tests.rs layout difference; leave it alone. Owned capi sources fmt to recheck.
- Final stable-source qbuild all PASS on 6e4c3c165: workspace 6878 passed, 0 failed, 0 ignored; clippy 0 diagnostics; native qbuild-20261024-023840-build-ctest PASS 197/197 build edges (101.4s), CTest 72/72 (145.1s), total native 250.0s. Owned capi fmt passed. No source changes after this gate began.
- Run 375 weather normal clean through 2032-01-01, ref_warnings=0, `gwp6-weather-2032-normal-a 1/73`; same first one-ULP wind difference; WE1 updated.
- Combined run 376 gwp6-both-2032-a terminal clean through 2032-01-02, ref_warnings=0, weather 1/73 (WE1), propdev 22/72 monthly projections with zero differences. No complete propdev operation verification: 50 months have unavailable custom-selector inputs; creation outcomes/types and removal identities still need full transcript replay. These are implementation gaps already specified by PV14, not a missing contract; do not file misleading spec questions for them.
- New diagnostic-driven milestone: FC4PropDevTranscript.h records genuine native selector entries/returns exactly once, preserving ordered duplicates, coordinates, requester identity, nested depth, purpose, zero returns, and original dispatch after restoration. Integrated only as capture; replay explicitly pending in log. Native synthetic test covers ABI, restoration/reuse and delayed callbacks. No original is called from shadow to get an oracle result.
- Native verify session 57620, qbuild-20261024-025148-build-ctest OUTSTANDING. Source changed during it to fix delayed-callback dispatch lifetime; intermediate evidence only. Collect same operation; final stable qbuild all is required again after this diagnostic-driven source change.
- Next: finish transcript replay against frozen pre-operation host inputs; do not count matching saved records alone as exact. At minimum run latest capture under both keys to 2032, normal too if turbo-specific. Preserve previous full-gate and game evidence separately.
- No game request outstanding. Summary script build-runs/shadow-summary.py prints counts and next free run number (377 after run 376).
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-weather-propdev6-r1-manifest.json.
