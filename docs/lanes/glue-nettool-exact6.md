| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-nettool-exact6 | Codex / `glue-nettool-exact6` | ready for review; gate PASS, clean game, shadow 8/14 | NT33/NT34 answers and live subclass hosts; preserve default-off paths. |

## Report

Round 3 did **not** reach 14/14. It adds measured replay inventories and resource
boundary diagnostics, plus a tested bounded powerline selection ABI. The six
city differences remain visible; neither controlled vectors nor loaded
resources are credited as exact city placement.

### Changes

- Merged main into the reserved detached checkout as requested. Shared refs
  were not moved; Git's packed-refs cleanup warnings did not prevent commits.
- Preserved the inherited NT30 implementation and its four flat price fixtures.
  NT30 explicitly describes these as controlled inputs, not native city snapshots.
- Added pre/post-refresh inventories for each replay solved entry and every
  corner: old/final index, provenance, smoothing membership, run/intersection/
  placed/connection bytes, vertex/lookup ids, old/retained corners, network and
  terrain hit/value, and live native terrain at getter time.
- Added a once-only read-only resource audit through the game's resource manager:
  real powerline tuning, raw PowerPoles entries and road bridge exemplar data.
  It does not populate the bridge geometry/model host or create pole occupants.
- Added `fc4_powerline_select`: bounded selection/count/retained-price replay
  over explicit read callbacks, cells, step ranges, helper prefix and tuning.
  It refuses missing callbacks, occupied/unavailable input cells, invalid ranges
  and insufficient output capacity. It does not claim placement, reuse,
  rendering, wire or curve coverage.
- Added `fc4_netgeom_get_smoothing_cells` for the NT30 diagnostic population.
  **C ABI exports changed; FC4_ABI_VERSION was not bumped.**
- New diagnostics run only under the opt-in nettool shadow. Replacement nettool
  remains disabled by default; no non-exact path was promoted into the house set.

### Verification

- Focused C ABI fixture: **1 passed / 0 failed**, session 67818 collected to
  termination. Explicit flat NT31 input selects (4,32),(14,32),(16,32):
  3 poles, 10 wire cells, valid=13, failure=0, price=65.
  Changing spacing to 20 produces 2 poles, 11 wire cells, price=52.
  Missing ray callback, unavailable cell and invalid range are refused.
- Explicit owned-file fmt: **2 files clean**. Initial relative-path invocation
  was rejected by the wrapper; absolute owned paths passed.
- Final `qbuild all`: **PASS**, Rust **6908 passed / 0 failed**, clippy
  **0 diagnostics**, native build **219/219**, CTest **74/74**.
  Session 66091 was collected to its terminal result; no duplicate check.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
  Earlier round-2 failed and corrected evidence remains in `build-runs/r2-*`.
- Fresh game: **nettool-exact6-r3-20261024-7c9f**; clean through **2032-01-16**,
  simulation 30.328 s, wall 41 s, slot 4, refs/exceptions/stalls/rtc/overruns all 0.
  Waited with one event-based local watcher operation, session 68010 terminal;
  no qwait and no duplicate request.
- DLL SHA256:
  `3ad54edbd75795668b20c3d08f6389944b7372937b37eee3bb80304251ec7b9b`.
- Exactly **14 counted nettool shadow lines, 8 exact**. This matches inherited
  normal run 383's 8/14; these six differences are not turbo-only.
  The run record is [run 400](../runs/nettool-exact6-r3-20261024-7c9f-417.md).

### Measurements and remaining work

| Probe | Native/core cost | Final entries/corners | Changed terrain vertices |
|---|---:|---:|---:|
| Straight road | 110/113 | 11/44 | 12 |
| Diagonal road | 170/192 | 17/68 | 34 |
| Shared-endpoint road | 90/98 | 9/36 | 18 |
| Straight street | 55/61 | 11/44 | 12 |

Each final inventory has the same pre-refresh entry count and smoothing count;
all final entries came from the solve, with no appended position/terrain
neighbours. Approximate absolute retained/live corner delta sums from diagnostic
text are 32.997318, 218.549516, 76.532530 and 63.342398. These are observations,
not corrections. NT34 includes a representative vertex and asks for a nonflat
constraint/height vector to distinguish solve and refresh.

Actual powerline tuning loaded as spacing=10, clearance=7, pole_cost=15,
wire_cost=2; the network INI has 16 pole mappings. All 9 road bridge exemplars
loaded with real deck ids and spans. This proves the resource boundary only.
Powerline remains valid13/0, cost65/0; bridge remains valid17/0, cost974/0.
The bounded selection ABI is not yet called by a complete live placement host.
Bridge geometry still has no populated type/selection/model host. These remain
NT31 integration work; no public totals were injected as replay outputs.

Questions appended this round: **NT33** (live clearance-ray/blocked-cell ABI and
fresh prefix), **NT34** (nonflat solve/refresh fixture), both in
`docs/specs/questions/network-tools.md`. Inherited **NT32** remains open.
The generated question index was not edited or regenerated by this clean-room
lane; the coordinator can regenerate it through its authorised workflow.

## Handoff

- Goal remains 14/14 counted city shadows; final round stopped at 8/14, clean.
- Real code milestone: 2a750c858; gate milestone: c3c87ea70.
- Final gate all PASS: cargo6908/0, clippy0, build219/219, CTest74/74, fmt2 clean.
- Fresh unique game label nettool-exact6-r3-20261024-7c9f completed clean, refs0.
- No asynchronous operation or game request remains outstanding.
- NEXT: answer NT33/NT34, then wire actual read callbacks and subclass placement/model hosts.
- Do not replace missing rays/classifier inputs with clear answers or fit city costs.
- Replay inventory/resource evidence is in build-runs/game-results/<label>.log.
- build-runs/summarize-r3.py extracts bounded measurements; watcher script is event based.
- Detached HEAD is intentional; supervisor publishes commits to the reserved lane branch.
- Ready queue means reviewable diagnostics and fixtures, not 14/14 or whole-owner credit.
