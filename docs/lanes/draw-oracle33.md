| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| draw-oracle33: driver/UI input oracle; tools | Codex / .claude/worktrees/draw-oracle33 / draw-oracle33 (detached HEAD) | ready; code ef7a8f212, fb4740a67, 81f7f0bd3, 29d5b9ffc; C ABI unchanged; seam cIGZGDriver/UI observer; pytest 401; cargo 8409, clippy 0, ctest 147/147; a4/b4 clean, byte-exact 20/20 frames; shadow n/a; questions none | 1. Measure DX7/OpenGL on an available device. 2. Define paused UI stopwatch advancement for the fixed schedule. |

- Capture key: `draw_capture = 1` (default off); `FC4DrawCapture.*` taps all 83 slots in all three version-checked driver vtables, only in memory.
- `draw_capture_scenarios = 1`, `draw_capture_frames = 2`, `draw_capture_warmup = 12`; primary stream `<LogPath()>.draw.jsonl`; optional `draw_capture_export` preserves it before slot reuse.
- Records scalar words, bounded input hashes, draw-time vertex/index ranges, texture rows, matrices, and one preorder UI tree (IDs, rectangles, visibility, caption hashes) per framework tick.
- Output-only pointers are not read as inputs. Unknown extents/read failures refuse completeness. This is native input determinism, not core/native equivalence or pixel equality.
- `tools/drawcheck.py LEFT RIGHT` uses only the standard library; first difference identifies frame/call/method/argument or buffer. Exit 0 exact, 1 different, 2 invalid; truncated/empty evidence cannot pass.
- Run/queue scripts preserve streams alongside copied logs. The existing build-layout test now accounts for the already-separate `host_crt` Rust suite.
- Final `python -m pytest tools/tests -q`: `401 passed in 361.26s`; includes a fresh final-source native build and `147/147` CTest pass (91.54s).
- `qbuild.py --wt <WT> all` ran once at 81f7f0bd3: `passed=8409 failed=0 ignored=0`, clippy `diagnostics=0`, CTest `147/147` (87.7s), `qbuild: PASS`.
- After the camera-hold correction, final-source `qbuild.py --wt <WT> quick`: build `5/5`, `qbuild: PASS`; final pytest above rechecked full native tests. Rust sources did not change.
- Earlier compiler-template and return-channel failures were corrected; intermediate pytest runs were 398/400 and 399/400 before 400/400 and final 401/401.
- Real inputs: Konradshohe save, native simulator/budget, `schedule = fixed`, seed 33, explicitly paused, loaded view position, zooms 1–5 each at rotations 0 and 1; two frames/camera, twice.
- Runs `draw-oracle33-a4`/`draw-oracle33-b4`: both `AUTOTEST VERDICT: clean`; DLL sha256 prefix in both verdicts is recorded with each run. A4 watchdog stalls 0; B4 1 (6.640s during zoom-1 capture), foreign exceptions 0 in both.
- Hardware initialization dialogs occurred in both. All `1,110,552` recorded calls/run are `software`; DX7/OpenGL device execution was unavailable and remains unmeasured.
- Real command `python tools/drawcheck.py build-runs/draw-oracle33-a4.draw.jsonl build-runs/draw-oracle33-b4.draw.jsonl` printed `IDENTICAL frames=20 calls=1110552 draws=99358 ui_frames=20`.
- Both completed streams: `incomplete=0`, 167,527,573 bytes; SHA-256 `5221BCBBAA4A8FD16446E59ADDD6298CBAA1AF369040626872EDCF8D782E1EF2`. Queue copies under `build-runs/game-results/` have the same hash.
- Initial a1/b1 captures stopped at 2/20 frames and drawcheck correctly printed `INVALID ... capture reports incomplete inputs`; a2/b2 and a3/b3 waited on the camera hold and produced no completed sweep. None count as exact captures.
- Cause: camera/refresh operations start a 2000ms hidden simulation pause (view3d-window 6f); it did not expire under the paused fixed schedule. Nested camera ticks also require a reentry guard. Final capture records the extra hold instead of waiting for it, retaining normal UI camera setup.
- Schedule would need a defined paused-framework-frame clock/stopwatch advance so that native camera hold can expire. `FC4Schedule.cpp/.h` are unchanged; no reference-store keys were altered.
- Additional checks: known ABI argument-word counts match all three 83-slot tables; both PowerShell scripts parse with 0 syntax errors. No questions filed; no C ABI export changes; no renderer/UI coverage credit claimed.
