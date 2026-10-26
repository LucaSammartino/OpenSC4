| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| automata-actions30: TV4 and network action boundaries (engine) | Codex / automata-actions30 / detached HEAD; reserved branch automata-actions30; code f7b408f9b, c893a0eb3, 8c6c55931; ABI false; seam 0x4E540001 | wip; Rust 8011/clippy 0/CTest 130/130; s2 216/216 then corrupt shutdown; s1 stalled 27/216; shadow exact 73/73; AP20 filed | 1. Complete construction SetLot/dust ownership before publication. 2. Resolve AP20 native wait. 3. Rerun clean s1/s2 216/216 and shadow, then enqueue kind engine. |

Merge criterion PASS: automata-actions29 already contained; merged main as 35809d64e, preserving both additive compat tables. Detached HEAD retained.
TV4 resolved from terrain.md section 15.5.12.2: version-checked, idempotent terrain StaticInit populates texture inputs before the retained native view's first Init (f7b408f9b).
Height-message fix: keep packed data1/data2 intact; SetVoid1 had overwritten X bounds with the sender pointer. Retain the sender only in separate IGZUnknown (c893a0eb3).
Network demolition fix: convert network tokens through borrowed AsOccupant for messages, manager callbacks and Shutdown; retain the synchronous message's sender (8c6c55931).
Focused tests PASS: plugin_ini_terrain_shadow_test and automata_construction_test; fixture covers static-init ordering/failure, fresh post payloads, distinct occupant receivers and message lifetime.
Final gate PASS: `build-runs/qbuild-test.log` 8011 passed/0 failed/0 ignored; `build-runs/qbuild-clippy.log` exit 0/0 diagnostics.
Final native gate PASS: `build-runs/fc4verify-build.log` 31/31 edges; `build-runs/fc4verify-ctest.log` 130/130. All reruns followed code fixes; no Rust files changed.
Game logs/verdicts below are in main's `build-runs/game-results/`; requests are in main's `build-runs/game-queue/done/`; unchanged house replace list.
Final artifact for all e runs: SHA256 `2a82f565fd84d2bb1b4bb4db0ef047806bc69ff5e05dccd08566a90221e8b152`; no subsequent build or code edit.
s2 actions PASS: `aa30-tv4-house-s2-e.log` has 216 completed actions; final mark at line 15473; old 84/216 boundary passed; end date reached 2028-01-30.
s2 verdict FAIL: matching `.verdict` corrupt, foreign_exceptions=1, rtc=0, ref_warnings=0; log 17522 faults at native 0x005D0F94 during occupant-manager drain.
s2 remaining ownership: construction occupant 03160B78, type 096E763C, has null dust effect; network-manager.md "What 0x005D0F94 actually requires" mandates completed SetLot/building/dust setup before publication and native Shutdown.
s1 criterion FAIL: `aa30-tv4-house-s1-e.log` / `.verdict`, stalled 27/216 on 2026-02-22; foreign_exceptions=0, rtc=0, ref_warnings=0; watchdog outside FreeCity4 at win32u.dll+0x11EC, zero main-thread CPU progress.
AP20 filed in `docs/specs/questions/application.md`: identify the s1 native wait/wakeup boundary, saved returns 00923A61/0090CF71 and required control capture; coordinator regenerates the question index.
Shadow criterion PASS: `aa30-tv4-shadow-e.log` / `.verdict`, clean through 2032-01-01, ref0; line 11215 reports 73/73 exact, 438 components, 730749 bytes, 0 mismatches.
Earlier-source failures kept distinct: s2-c 27/216, corrupt native height copy 00756A50 (cc9e4c2d); s2-d 27/216, exited with four occupant-receiver RTC reports (9d89588a); both corrected before e runs.
NM20-A stays an uncollected runtime observation: original control, native placement result and first pointer/cache write; passing action count does not supply that capture.
Renderer limits carried forward: mirrored normals are core face normals; texture-layer population is incomplete. No terrain or draw bypass was introduced.
Not enqueued for landing: final game verdicts fail acceptance; local merge entry is wip despite the passing gate.
Next: complete valid construction setup/dust ownership, resolve AP20, then rerun clean s1/s2 216/216 and final-DLL shadow before engine enqueue.
