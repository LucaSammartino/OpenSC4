| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin-close12 (cSC4View3DWin closure) | Codex / `core-view3dwin-close12` (engine lane) | in progress | Collect r4 focused run; finish formatting, tests and closure table; run all/verify. |

## Handoff

- Goal: finish owner closure and all/verify; guard OVER at 151k, no queue entry.
- r4 resumed 5dfaa9c60, detached HEAD. First required merge: already up to date;
  VW20 is an ancestor. Do not change shared refs. Prior evidence is below.
- Commit bc7427d27 adds literal zone/network/fixed-lot command dispatch with city
  guards, fixes double UI toggle, initializes city animation/model-maker objects,
  enforces required main-panel lookup, balances replaced GUI image/surface refs.
- Adds tutorial follow-up including inert buttons, settled catalog identities,
  reward-unlock propagation, owner global-terrain dispatch, destruction-only +410.
- Adds closure_round4_tests.rs: command/control lifetimes, T36 fresh fallback,
  required/optional creation, edge/capture branches, mayor bookkeeping, catalogs,
  script/vehicle/disaster identities and ordered shutdown. Existing expectations
  were updated for tutorial callbacks, terrain dispatch and destruction-only ref.
- NEXT FIRST: collect native focused exec session 74977 to its terminal result;
  last poll still running. Log build-runs/focused-close12-r4-native.log. It began
  before later source edits, so it is diagnostic evidence, not a final-source pass.
- Cargo command: cargo test --manifest-path engine/Cargo.toml -p fc4-core
  ui_windows::view3d -- --test-threads=1. Shared machine has many rustc processes.
- q.py first launch failed Git Bash CreateFileMapping (Win32 error 5); preserve
  focused-close12-r4-console.log. No r4 focused terminal result or all/verify yet.
- qbuild fmt found 495 diff lines; applied check-generated hunks with apply_patch,
  then more source/tests changed. fmt-patch-close12.py names nine files; add
  lifecycle to its list, apply its output with apply_patch, recheck explicit files.
- README still retains 14 settled audit deficits; update only after validation.
- No new questions, ABI/capi/plugin edits or game requests. Full closure unclaimed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin-close12-r4-manifest.json.

## Previous report

Merged `re-view3dwin-vw20` first, fast-forwarding to `c13d5f600`. Implementation is
`18db8cb78`. This patch closes VW19, not the entire owner. The supplied closure
table had substantial settled implementation gaps beyond its three VW19 entries.

Implemented column module paths: `engine/core/src/ui_windows/view3d.rs`,
`engine/core/src/ui_windows/view3d/state.rs`, and
`engine/core/src/ui_windows/view3d/references.rs`. Both logical and opaque-reference
record adapters now assign trigger pairs into the existing table in stream order.
Absent keys survive, duplicate counts overwrite without moving nodes or growing
capacity, empty streams preserve the table, and only new keys use ordinary growth.
Loading does not pre-size, assert counts, or send trigger notifications.

`engine/core/src/ui_windows/view3d/trigger_load_tests.rs` implements VW-T40 through
VW-T44, each through both adapters: collision round trips, duplicate zero/MAX
assignment, populated/empty loads, pair-count versus distinct-key capacity, and
growth-time collision order. All five tests passed. The module README resolves
the three VW19 entries and names the implementation and tests.

## Coverage audit and next work

The existing 34-row Owner closure table maps sections to code and tests, but 19
rows explicitly retain settled implementation/test deficits. Complete coverage of
the other rows and full owner closure cannot be certified by this run. There are
no remaining spec-question entries; the deficits are not Unsupported contracts.

Implementation/test gaps remain in view3d-window sections **1.1, 2, 4, 5.1, 5.2,
6, 6.3, 6a–6e, 6g–6j, 6l–6m, and 7**. The README retains the individual next
steps: override/control adapters, dormant fields and teardown, required fallbacks
and graphics QI, GUI extras, edge/capture branches, command/button/menu routing,
mayor bookkeeping, mission/advice destinations, hidden-BMP visibility, and helper
integration. VW-T26/T27/T29/T32/T33/T34/T36 still lack complete coverage. These
settled gaps must be finished before claiming the owner; no new spec question is
justified by them.

## Validation

- Ran `qbuild all` once on final Rust source. Its parallel Rust phase reported
  590 passed / 2 failed in unchanged C API occupant-manager cross-thread tests.
  Both depend on a worker running within 300 ms; slot44 observed zero lock waits
  instead of one. Clippy passed with zero diagnostics. Queued native job
  `qbuild-20261024-182656-build-ctest` passed build 262/262 edges and CTest 81/81.
  Preserve the failed run: `build-runs/all-close12-console.log` and
  `build-runs/qbuild-test-parallel-first.log`.
- Changed scheduling input only to `RUST_TEST_THREADS=1` after that diagnosis and
  reran the workspace Rust phase with `qbuild test`: **7394 passed / 0 failed /
  0 ignored**. Both formerly failing tests and all five new vectors explicitly
  passed. No source changed between these runs. Evidence:
  `build-runs/test-close12-serial-console.log` and `build-runs/qbuild-test.log`.
- Ran the explicitly requested `qbuild verify` through the coordinator queue:
  `qbuild-20261024-183418-build-ctest`, verdict **pass**, build **46/46 edges**
  (2.0 s), CTest **81/81** (90.2 s), total 94.4 s. Evidence:
  `build-runs/verify-close12-console.log`, `fc4verify-build.log`, and
  `fc4verify-ctest.log` under `build-runs/`. Every operation was collected to its
  actual terminal result; none remains outstanding.
- Explicit formatting check passed for `trigger_load_tests.rs` and `state.rs`:
  2 files, zero diff lines (`build-runs/fmt-close12-final-console.log`).
- The earlier focused compilation was interrupted under shared contention and
  produced no test verdict; its separate incomplete evidence remains in
  `build-runs/focused-close12.log`.

No game runs or shadow measurements: this is a core-only patch, without plugin
or C ABI wiring as requested. No C ABI exports or ABI version changed. No new
questions were filed; VW19 was settled by the merged spec. The READY queue entry
is for this passing partial patch, not a claim of full owner coverage.
