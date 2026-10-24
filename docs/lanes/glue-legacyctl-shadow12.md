| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-legacyctl-shadow12 (legacy control shadow) | `Codex` / `glue-legacyctl-shadow12` (engine lane) | ready, owner-only portion: cargo 7375/0, clippy clean, CTest 81/81; run 520 clean through 2032, legacyctl 72/72 exact; LC14/LC15 open; public ABI unchanged | 1. Coordinator review partial coverage and integrate regress profile. 2. Resolve LC14/LC15 before full build replay. |

## Report

Default-off `legacyctl_shadow=1` observes published native primary QueryInterface/AddRef/SetOwnControl and legacy GetControl/SetOwnControl vtable boundaries. Rust replays from pre-call state using the existing core. All five owner fields (references, control, parent, procedure, ownership), scalar returns and QueryInterface output identity are compared. No native post-state enters Rust. QueryInterface output bytes stay opaque on both success and rejection.

SetParameters calls are counted and a post-build GetControl probe is measured separately. Full property/per-type interpreter replay is explicitly excluded. LC15 requests the observable direct-call boundary contract; LC14 requests ForceControl/UseWinProc return contracts. These methods remain unhooked. This is partial closure, not verification of the full subsystem.

Implementation: `971d183e3` owner replay/transport/hooks; `021be6fe6` opaque QueryInterface output fix. The private static-link transport adds no public DLL export; FC4_ABI_VERSION unchanged.

## Validation

Final stable-source `qbuild.py --wt <WT> all` completed PASS: cargo 7375/0, clippy 0 diagnostics, native 159/159 edges, CTest 81/81. Coordinator queue: `qbuild-20261024-184451-build-ctest`. Evidence: `build-runs/legacyctl-final-all-console.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Focused tests 2/0; explicit owned Rust fmt passed. No source changed after that gate, so no duplicate gate launched. Pre-QI-fix evidence remains in `build-runs/legacyctl-pre-qi-fix/`.

Run [520](../runs/legacyctl12-owner-2032-971d183e3-r1-640.md): `legacyctl12-owner-2032-971d183e3-r1`, Konradshohe, turbo to 2032-01-01, enable empty, legacyctl_shadow=1. Existing completed verdict collected without duplicate request. Clean, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Final date 2032-01-01, 28.875 real seconds; DLL SHA256 prefix `0c53e5ea`.

Exact line: `legacyctl12-owner-2032-971d183e3-r1 72/72` (shadow label `legacyctl 72/72`). All monthly lines, 2026-02-01 through 2032-01-01, end mismatches=0. Final cumulative compared=7976, builds=775, build_getters=775, addref=3270, own=775, control=1564, qi=2367. No normal-speed diagnostic needed. Counts computed directly from log; coverage tools were not opened or run.

Evidence: `build-runs/game-results/legacyctl12-owner-2032-971d183e3-r1.log` and `.verdict`; consumed request in `build-runs/game-queue/done/`. No request outstanding.

## Questions and integration

LC14/LC15 filed in `docs/specs/questions/legacy-control.md`; no new questions this continuation. Full build replay remains open; the measured owner portion is ready. Coordinator must review partial coverage before crediting. No coverage map/shadow table edited.

Regress profile integration deferred to coordinator: this lane cannot open/edit tools/regress.ps1, merge main, or change detached-HEAD refs. If accepted, append legacyctl_shadow to the current single shadows profile string. Keep default-off behavior.
