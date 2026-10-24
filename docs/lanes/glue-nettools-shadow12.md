| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettools-shadow12 (network tools draw shadows) | `Codex` / `glue-nettools-shadow12` (engine lane, detached HEAD) | READY partial: source aeaff8388; ABI unchanged; Rust 7293/0, clippy clean, CTest 81/81; runs 512 turbo and 513 normal clean to 2032-01-01, ref_warnings=0; nettool wide drag shadow 73/73 in both, nettool drag shadow 0/73 (NG8) | (1) answer NG8 and existing NT35/NT36; (2) coordinator inventory/coverage review and question-index regeneration; (3) coordinator may append only accepted exact key to current regress shadows string |

## Report

Added `FC4NetToolDragShadow.cpp/.h` with independent default-off ini keys
`nettool_drag_shadow` (single-cell-width types) and `nettool_wide_drag_shadow`
(wider types). Both run at city load and each month on private original tools.
They compare slot 18 return, slot 23 ordered coordinates, and slots 26/27 counts
against core geometry replay. Eight endpoint pairs and both snap modes cover
axis/reversal, one-cell, pre-adjust, staircase and snap cases across all 13 types.
The host whitelists dimensions only; no placement, validation, terrain writes,
native field reads, shared tool mutations or native results fed back into core.
Private tools take AddRef and use FC4_RELEASE_CREATED; SDK vectors die before
the city allocator does. Replacements are rejected by the probe. Setup failures
and unavailable observations count against exactness.

This is partial public draw projection, not verification of an entire owner.
The wide group (types 2, 6, 12) executes 48 vectors per observation, 3504 per run.
The narrow group executes 160 per observation, 11680 per run. Existing scripted
`nettool_shadow` remains intact. No solved pieces, cost, bridge/model, pole/wire,
commit, refit or resource-loader credit is claimed. Existing NT35/NT36 cover
retained solve inputs and subclass/model integration; those questions were not
duplicated. The full subsystem remains partial.

The requested owner-by-byte inventory from `tools/coverage-map.tsv`, coverage
tool execution, and `tools/regress.ps1` profile edit conflict with the explicit
clean-room ban on any `tools/` access. None were accessed or modified. Coordinator
must supply/review that inventory and credit. Both keys remain default-off. Only
`nettool_wide_drag_shadow` is an exact candidate for the coordinator's current
single shadows string, if this explicitly limited scope is accepted.

## Gate

Final source is aeaff8388 (implementation 47f183c04 plus ABI citation).
`qbuild.py all` PASS: Rust 7293 passed, 0 failed, 0 ignored; clippy 0 diagnostics;
queued native build 47/47 edges (2.4s), CTest 81/81 (92.2s), total 98.7s.
Native queue: `qbuild-20261024-160235-build-ctest`; collected to terminal.
No duplicate verify was launched: all already handed the native checks to the
coordinator. Logs are `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log`, `fc4verify-ctest.log` and the matching build-results verdict.
The initial gate had CTest 80/81: Init's specified AL result needed an ABI review
comment. That gate also detected a documentation commit during its run. Failure
evidence is preserved in `fc4verify-ctest-r1-failed.log` and the
`qbuild-20261024-153703-build-ctest.verdict` result. Corrected gate ran with a fixed
HEAD. No Rust files or C ABI exports changed; ABI version was not bumped.

## Real-city results

Both unique queue operations were collected to terminal with a single file-event
wait per run, without qwait. Both used Konradshohe, original nettool/netmgr,
enable=simulator, and both new ini keys. DLL SHA256 prefix: `76cfa9bc`.

- Run [512](../runs/nettools-shadow12-draw-2032-20261024-aeaff8388-512.md):
  `nettools-shadow12-draw-2032-20261024-aeaff8388`, turbo, clean to 2032-01-01,
  real_seconds=31.094, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0,
  rtc=0, overruns=0. `nettool wide drag shadow 73/73`; `nettool drag shadow 0/73`.
- Run [513](../runs/nettools-shadow12-draw-normal-2032-20261024-aeaff8388-513.md):
  `nettools-shadow12-draw-normal-2032-20261024-aeaff8388`, normal, clean to
  2032-01-01, real_seconds=110.390, same zero problem counters.
  `nettool wide drag shadow 73/73`; `nettool drag shadow 0/73`.

Exact shutdown lines in BOTH logs:
`nettool wide drag shadow: city shutdown: exact 73/73 scope=draw-only`
and `nettool drag shadow: city shutdown: exact 0/73 scope=draw-only`.
Every wide observation has mismatches=0, unavailable=0. Every narrow observation
has mismatches=6, unavailable=0: type 4 `(16,44)-(10,40)`, both snap modes,
native/core cells 12/10, total 7/6, straight 0/0, return true/true. Each case has
two cell differences and one total difference. No turbo-only exemption.
Full evidence stays under `build-runs/game-results/<label>.log/.verdict`.

## Questions and next steps

NG8 appended to `docs/specs/questions/network-geometry.md` asks for the underground
auto-connect occupant selection body needed by the omitted extension host input.
Archived NG1-NG7 and relevant answered NT28 were checked to avoid duplication.
Coordinator must regenerate the question index; this lane cannot run tools scripts.
After NG8, implement that host dependency and repeat the same matrix; do not
calibrate to native totals. Full owner work also needs existing NT35/NT36.
No build or game request remains outstanding. READY queue is for this measured
partial delivery only. Supervisor publishes detached commits to the reserved lane
branch; this lane did not change shared refs or merge main.
