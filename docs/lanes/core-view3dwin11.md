| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin11: cSC4View3DWin core | Codex / core-view3dwin11 | ready; partial owner closure; all/verify PASS | Review and merge tested partial implementation; claim only closed README rows |

## Report

Implemented column: `engine/core/src/ui_windows/view3d.rs` and
`engine/core/src/ui_windows/view3d/`. The module README contains the authoritative
Owner closure table, with code items, tests and remaining work by spec section.
This is a tested partial implementation, not complete cSC4View3DWin owner closure.

The core models camera/view requests, scroll and zoom state, input/control routing,
notifications, lifecycle, panels, menus, status, save references and ordered paint
through host traits. Picking and projection use the renderer/host interfaces.
The preceding implementation milestone is c9dc50ae0; this continuation fixes the
deferred queue's shutdown ownership and completes additional settled test vectors.

- 92c78d150 releases every queued notification reference in `shutdown_tail`,
  including when shutdown repeats; adds modal, locale, date and graph vectors.
- d084e399b corrects the queue fixtures to explicitly acquire their second owned
  entry: only BudgetFailure is a notification producer for this queue.
- The existing b4b0756a4 zoom fixture correction is verified by the final gate.
- README rows now identify separate identity, override and focus sections;
  locale parsing, modal drop/recheck, date boundaries and graph/stop vectors
  have explicit coverage. No settled implementation gap is relabeled Unsupported.

## Verification

Stable final engine source passed both requested gate operations, collected to
their actual terminal results; no duplicate progress checks were launched.

| Check | Measured result | Evidence under build-runs/ |
|---|---|---|
| Workspace tests | PASS: 7,155 passed, 0 failed, 0 ignored; sum of all 45 result summaries | `qbuild-test.log` |
| City-view tests in workspace gate | PASS: 74/74, including shutdown queue balance, safe/disabled modal drop, comma-locale SizeOf, VW-T21, VW-T24 and VW-T39 | `qbuild-test.log` |
| Clippy, warnings denied | PASS: exit 0, diagnostics 0 | `qbuild-clippy.log` |
| Explicit qbuild verify | PASS: build 46/46, CTest 81/81, 0 failed; 90.5 s total | `verify-resume.log`, `build-results/qbuild-20261024-135533-build-ctest.verdict` |
| qbuild all, including native build/CTest | PASS: build 46/46, CTest 81/81, 0 failed; native 85.0 s | `all-resume.log`, `build-results/qbuild-20261024-141439-build-ctest.verdict` |
| Scoped formatting | PASS: lifecycle.rs, owner_tests.rs, resolved_tests.rs; zero differences; corrected fixture checked again | qbuild fmt |

Failed diagnostic evidence is preserved separately: `focused-1.log` (67 pass /
1 stale zoom-fixture failure) and `focused-resume.log` (69 pass / 2 queue-fixture
failures). The final workspace run verifies all corrections. The earlier native
request `qbuild-20261024-131859-build-ctest` also passed 81/81 but reported a changed
tree; it is supporting evidence only, not the final gate.

Game: n/a, core-only work. No game requests were made or remain outstanding.
No plugin/capi wiring, ABI export changes or ABI version change.

## Questions and remaining closure

VW19 is filed in `docs/specs/questions/view3d-window.md`: the spec does not settle
trigger reconstruction order or duplicate saved keys. The code explicitly uses
literal stream-order insertion. No additional question was needed in this round.

The following view3d-window sections remain partially open as implementation or
test work: §1.1, §2, §4, §§5.1–5.2, §6, §6.3, §§6a–6e, §§6g–6m and §7.
The open save reconstruction contract also affects view3d §5.3 (VW19).
Specifically, remaining work includes unified override adaptation, dormant helper
active flags and panel extras, required fallback/failure paths, complete
mission/advice/background vectors, coverage/graph owner integration, remaining
menu/command dispatch cases, hidden-BMP visibility callbacks and exhaustive input
vectors. VW-T26/T27/T29/T32/T33/T34/T36 are not fully covered. These are enumerated
in the README; complete owner coverage must not be claimed.

Other owners in view3d.md (renderer, terrain view, effects and spans) are outside
this implementation. The window invokes one renderer draw; pass ordering within
that draw belongs to the renderer.

Next: coordinator reviews this partial closure and merges the READY entry if
accepted; future work completes the listed README obligations and resolves VW19.
No coverage-map rows were edited.
