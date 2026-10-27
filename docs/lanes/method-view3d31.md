| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-view3d31: View3D method observations; kind engine | Codex; method-view3d31 worktree; detached HEAD, reserved branch method-view3d31 | continue: code bd363ed0a, b33024286; ABI unchanged; gate PASS; house exact 357/357; reach3 pending; questions none | 1. Collect existing reach3. 2. Rank remaining methods and finish engine enqueue. |
Gate: qbuild all PASS: cargo 8255/0, clippy diagnostics 0, CTest 143/143; own Rust fmt 2/2. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-ctest.log`; quick previously 6291 tests/353 native edges PASS.
House: `regress-method-view3d31-house1-shadows`, clean at 2026-04-01, ref_warnings=0, 7 methods/357 complete calls, mismatches=0, resyncs=0; log `C:/Users/tlori/FreeCity4/build-runs/game-results/regress-method-view3d31-house1-shadows.log`.
Follow-up: reach2 rejected before game (unsupported top-level close_dialogs); corrected reach3 is running in slot 6 since 2026-10-27 00:03:01; two qwait timeouts, no verdict/reach file observed. Same DLL, house switches, dated camera command script, reach/startup/lifecycle on; not qualified evidence.
Table counts are cumulative HOUSE evidence only; slash-separated values follow method order; a single zero applies to each listed method. Empty/partial methods get no credit.
| Method/address | Bytes | Calls | Complete | Mismatches | Missing observation |
|---|---:|---:|---:|---:|---|
| AsIGZWin 0x005F8400 | 11 shared | 2 | 2 | 0 | - |
| GetRenderer 0x0043FD00 | 7 | 5 | 5 | 0 | - |
| GetModelMaker 0x007F0FE0 | 7 | 1 | 1 | 0 | - |
| CanZoomIn 0x007E5780 | 31 | 1 | 1 | 0 | - |
| CanZoomOut 0x007E57A0 | 31 | 1 | 1 | 0 | - |
| GetCurrentViewLevel 0x007E57F0 | 7 | 328 | 328 | 0 | - |
| GZOnCommand 0x007E5BA0 | 5 | 19 | 19 | 0 | - |
| ScrollStop 0x007E5740 | 60 | 42 | 0 | 0 | Writes exact; return contract unresolved |
| Notify DoMessage 0x007F53C0; window DoMessage 0x007F3E50; FilterMessage 0x007F58E0 | 1304/807/968 | 317/49/575 | 0 | 0 | Tutorial/base/dispatcher/UI effects |
| GZPaint 0x007F6200 | 518 | 245 | 0 | 0 | Timer/drawing/deferred-message effects |
| SetZoom 0x007E60E0; ZoomIn 0x007E62C0; ZoomOut 0x007E6320; SetZoomLimits 0x007E6150; SetRotation 0x007E61D0; RotateLeft 0x007E6220; RotateRight 0x007E6270; SetZoomAndRotation 0x007E6380 | 101/95/95/123/79/72/72/108 | 0/0/0/1/0/0/0/0 | 0 | 0 | Renderer/timer/hidden-pause/invalidation effects |
| ProcessCommand 0x007F1C10 | 5749 | 0 | 0 | 0 | Command/UI collaborator effects |
| GetCurrentViewLevelInfo 0x007E57E0; GetKeyAccelerator 0x007E56F0; GetTerrainQueryEnabled 0x007E5900; GetLastSelectedBuilding 0x007F1060; GetLastSelectedLot 0x007F1080; SetLastSelectedBuilding 0x007F1020; SetLastSelectedLot 0x007F1040; SetScrolling 0x007F64F0 | 15/7 shared/7/20/20/20/20/82 | 0 | 0 | 0 | No shadow entries observed |
Owner map: 201 functions/68,258 B; 6 owner functions/88 B complete (+11 shared B); 174 owner functions unbound, needing return/out/owner-write/collaborator observations. Initial largest: ProcessCommand 5749, AddGUI 4902, HandleButtonActivated 3273 B; reach ranking still missing.
Implementation: default-off view3d_shadow; entry-state core replay, exact owner/return/out comparisons and camera read capture; no sampling/resync. SPEC-GAP preserves incomplete returns for slots 20/34/53/54. No questions filed. Helpers/scripts are retained under ignored build-runs; no merge queue file written.
Next: qwait SAME `C:/Users/tlori/FreeCity4/build-runs/game-results/method-view3d31-reach3.verdict` (do not submit a duplicate). Collect log and `build-runs/view3d-reach3.tsv`; `build-runs/view3d-report.py` joins owner bytes, both logs and reach into per-method JSON rows. Rank/review every reached unbound method, finish measured handoff, then enqueue kind engine with actual HEAD; supervisor publishes detached commits.
