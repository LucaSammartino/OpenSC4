| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-replace23 (regional-city setter differential) | `Codex` / `glue-regionalcity-replace23` (engine) | ready for measured shadow changes only; cargo 7646/0, clippy 0 diagnostics, native build passed, ctest 93/93; rc23-r4-setters2032-20261025b 73/73 exact through 2032, ref_warnings=0 | Implement replacement owner/key and direct-construction integration; add registration/ownership tests and both requested replacement runs. |

Adds four isolated native setter phases compared against independently mutated Rust state, including exact serialized records: scalar/string setters, unknown commodities, negative commodity clamping, empty strings, tutorial state and growth-map insertion. The native fixture never calls Init or attaches to the city; its factory acquisition and created-object release are audited. Live-record preservation and point-list checks remain. Everything stays behind default-off `regionalcity_shadow = 1`.

Two new Rust tests passed. Workspace gate: 7646 passed, 0 failed, 0 ignored; clippy 0 diagnostics. Final native verification `qbuild-20261025-071633-build-ctest`: build 211/211 edges, CTest 93/93. Logs remain under `build-runs/`.
Game: `rc23-r4-setters2032-20261025b 73/73`; clean through 2032-01-10, ref_warnings=0. First/last observations decoded 85 records and compared 16008 fields with mismatches=0. The 2029 scripted full save returned ok=1.
Run artifacts: `build-runs/game-queue/done/rc23-r4-setters2032-20261025b.req` and `build-runs/game-results/rc23-r4-setters2032-20261025b.{verdict,log}`.
Preserved diagnostics: initial native ret_channel_check failure was fixed by citing the IsTutorial bool contract; `build-runs/rc23-r4-first-native-failure.txt` records it. Run `rc23-r4-setters2032-20261025a` was 73/73 exact but had 146 audit warnings; recording the factory acquisition fixed these in run b.

Still missing: the Rust-backed COM owner, `regionalcity` registration, direct-construction interception, registration/ownership tests, and regionalcity-only/full-house replacement runs. A SPEC-GAP comment notes that regional-city §1 names direct construction sites without an interception contract. No native replacement or replaced coverage credit is claimed.
No public C ABI exports changed. No questions filed or tools files edited.
