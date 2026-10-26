| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vh30 — VH1/VH7 vehicle specifications; kind docs | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/re-vh30; detached HEAD, reserved branch re-vh30 | ready; commits 2ed79e4df, 22e69230f plus this report; doccheck PASS; ABI unchanged; game/shadow n/a; VH1 answered, VH7 runtime-narrowed | 1. Capture VH7-R1/R2/R3. 2. Implement owner rules using the worked cases. |

- vehicles.md §12.16 completes Vehicle network displacement/reattachment and off-road anchor, yaw, height and obstacle-response rules; seven worked cases.
- Existing §§12.10/12.12/12.13/12.15 retain Vehicle/Train/Tank command, speed, derail, firing and scheduling rules; effects remain the Tank projectile boundary.
- vehicles.md §12.9 and automata.md §6.3 establish serialization receiver S=P+8 from QI, constructor and vtable evidence; record/live fields alias without changing stream bytes.
- Named leader, transport selector, network occupant, path key and segment index; preserved the option byte and route annotation without inventing meanings; added a worked load case.
- VH1 and VH7 moved to docs/specs/questions/answered/vehicles.md with dated DIFFERS answers; index regenerated: vehicles 0 open / 11 answered.
- VH7 is narrowed, not fully closed: R1 observes the first post-load geometry binding/index handling; R2 identifies the option byte's consumer; R3 traces the route annotation producer.
- doccheck check PASS; git diff --check PASS. Cargo/clippy/ctest and game/shadow n/a for this specification-only lane.
- questions.py check FAIL: nine errors in untouched files (four malformed IDs; five duplicate open/answered entries). No vehicle-entry error was reported; those other questions were not edited.
- Required index regeneration also corrects the existing automata open count from 4 to 9.
- Evidence used main-checkout export/decomp and instruction bytes; constants checked against the working PE. REA was not needed. No engine/, docs/abi/, game or decompiler output edits/commits.
- No ABI rows proposed; no ABI exports changed. Queue kind docs; no game runs.
