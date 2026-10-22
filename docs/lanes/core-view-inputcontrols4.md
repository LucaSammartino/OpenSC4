| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-inputcontrols4 (pure core view input tools) | `Codex` / `core-view-inputcontrols4` (engine lane) | ready: cargo 5003/0; clippy 0 diagnostics; ctest 68/68; 16 new tests; no ABI or game runs; VIC3-VIC7 filed | (1) Resolve VIC1-VIC7 before native adapters/full-owner claims; (2) regenerate question index during integration. |

## Delivered scope

New production `engine/core/src/view_input_controls/` implements the published
supported algorithms of view-input-controls.md sections 1-8/10: right-drag
scrolling, level terrain, label/sign tools, sign-label demolition and all three
filter predicates. Core owns threshold/angle/speed calculations, selection and
drag state, level rectangles/altitude selection, road/avenue snapping, modal
acceptance/filter policy, encoded-glyph editing and sign-position replies.

Typed boundaries expose primitive GUI/resource/input/world operations. Terrain
uses the existing Terrain API directly through TerrainAccess; projection uses
existing ViewUtilities/Pixel/Vec3. Camera rotation is a primitive query; view
scroll commands do not invent camera integration cadence. Cursor lookup and
window application are separate primitives. No native owner algorithm/output
is passed through a host. Getters use shared references, never RefCell.

Independent `fixtures.md` was written before implementation; 16 executable
tests check discriminating values, ordering, cancellation/deletion, glyph
budgets, filters, real core terrain mutation and view projection.

## Inventory and residuals

`engine/core/src/view_input_controls/README.md` lists every published method,
entry identifier and state-field offset for the five tools/three filters,
with implementation mapping and residuals. Method byte sizes/end ranges and
complete binary inventories are unpublished; byte coverage/full-owner claims
remain unknown (VIC5). Only specified base-support operations are implemented;
no full cSC4ViewInputControl/COM lifetime is claimed.

Sign initialization/shutdown support local tool state only: native service,
notification lifetime, QueryInterface and wire binding remain unsupported.
VIC1 makers/global-terrain other cases and VIC2 unknown service protocol have
no invented adapter. Unbound paste/invalid encoding/resource or numeric inputs
at checked boundaries return explicit Unsupported results.

New questions in `docs/specs/questions/view-input-controls.md`: VIC3 numeric/
edge/rounding boundaries; VIC4 encoded text/pending/newline rules; VIC5 full
method/byte inventory; VIC6 failures/lifecycle; VIC7 release while typing.
VIC7 retains the literal published "otherwise idle" rule. No question index
generation was run (task forbids tools/ and editing the generated index).

Excluded: player-drive, flora, torch, zone/network tools, command/UI dispatch,
the capped terrain owner, app UI invocation and game file mutation. No ABI,
plugin/default/registration changes, game requests, native-exact/shadow/replaced
or complete-owner claims.

## Measured validation and preserved failures

Final stable `qbuild all` on source commit `dc9c696c`: **PASS**, cargo **5003/0**,
clippy **exit 0 / zero diagnostics**, CMake **4/4** edges, ctest **68/68**
(213.0 s). Logs copied to `build-runs/final-pass/`; this report-only commit
does not change the tested source. All nine new Rust files pass `qbuild fmt`.
The shared lib.rs check requests broad existing module reorder/format changes;
its output is preserved in `build-runs/fmt-including-existing-lib.log`. Only
the additive export at the end of its list was edited; no mass format applied.

Baseline: cargo **4987/0**; later stages failed with tree changed during edits,
so no clean baseline full-gate claim. Logs: `build-runs/baseline/`.
Quick: early snapshot **2960/0**, dependents compiled, CMake **170/170**, but
tree changed; overlapping second quick **2975/0** (16 new tests), dependents
compiled, configure failed with Ninja recompaction Permission denied. Logs:
`build-runs/quick-overlap/`. The overlap was a lane execution error.

First stable full attempt (`2f41a9a8`): cargo **5003/0**, CMake **2/2**,
ctest **68/68**, clippy failed explicit_counter_loop in sign AppendText.
Preserved in `build-runs/final-attempt-1/`. The fix bounds input iteration by
33 minus initial glyph count, preserving skipped-newline budget, and the final
stable full gate above passed. No failing result was discarded.

## Commits and next steps

`558e8ba7` initial production/independent fixtures; `cdb14932` executable
fixtures/inventory/questions; `6aa2d027` primitive cursor boundary;
`2f41a9a8` focused results; `dc9c696c` clippy fix/failure record.
Resolve VIC1-VIC7 before attempting native adapters or expanding ownership;
integrator regenerates the question index. Pure core delivery is ready.
