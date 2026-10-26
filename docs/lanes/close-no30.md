| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-no30 network-occupants closures; kind engine | Codex / .claude/worktrees/close-no30 / detached HEAD; reserved branch close-no30 | continue; code ae083e7af, 99dad42b9, fbaca34c0; ABI exports unchanged; cargo 7966/0, clippy 0 diagnostics, CTest 129/129; game/shadow not run; questions none | 1. Finish base, pylon and underground-connection implementations below. 2. Run required shared-queue game shadow and full gate after changes. 3. Enqueue kind engine only when complete. |

- cSC4PowerPoleOccupant: implemented NO33/NO43; pool policy, occupant/local bounds, Pick, full Init/exemplar refresh, model positioning/destruction. README links existing wire/render tests and new worked cases 54/55/62/68/71/73.
- Pole tests: power_pole/pool.rs::native_pool_policy_conformance_62 and power_pole/model/tests.rs::{occupant_bounds_conformance_54_55,pole_pick_conformance_68,pole_init_guard_exemplar_refresh_and_destruction_conformance_71_73}; existing tests cover 65–70/72–73.
- Pole exemplar identity comes from the base holder, including loaded state; unchanged_exemplar_uses_base_holder_identity_including_loaded_state prevents a duplicate stale cache. Base holder owns its reference lifetime.
- cSC4PowerPoleOccupant::tConnection: implemented NO33/NO43; existing links/line_cells code and tests now cited for counted duplicate lines, ownership, unsigned enable, cell walk and wet refresh accumulation.
- cSC4PowerLineOccupant: implemented NO13; acquired setters/getters and shutdown forwarding plus existing live/record mapping. power_line.rs::line_layout_and_shutdown_forwarding_conformance_74 tests duplicate refs, equal-pointer acquire/release, base-call order and preserved fields.
- cSC4NetworkOccupant: partial; NO12 eight type rows/four entry bytes and NO14 rotation/flip slots implemented; tests.rs::{path_connections,orientation_flip_rotation,init_lifecycle}. Init now clears path-built and re-posts usable on re-Init.
- Base remaining: geometry/box/quad/parts/property/static/cache/render integration. NO32 numeric UV/zoom/triangle/normal/RGB helpers already exist in surface.rs with tests; integrate them rather than treating NO32 as an unanswered spec question.
- cSC4PylonOccupant: partial; NO10 identity already tested and README citation corrected. NO36 is answered in §§6.2.3–6.2.4/tests 52–53/56–59 but model/bitmap/label/Draw/message code still needs implementation.
- cSC4UndergroundConnectionOccupant: partial; implement settled UG5/NO40 §§3.6.4–3.6.6 pipeline/messages/draw and worked cases 60–61. Keep NO40-C open: descriptor yaw matrices, attached ceiling side argument and bounded descriptor/shade inputs require the stated runtime observation.
- NO30 already has code/tests in model/prebuilt.rs and tests_model.rs; its remaining stale README references need an audit, not an invented option exception.
- Corrected-source cargo: 7966 passed, 0 failed, 0 ignored; build-runs/qbuild-test.log.
- Corrected-source clippy: exit 0, 0 diagnostics; build-runs/qbuild-clippy.log.
- qbuild all: PASS; native build 35/35 edges, CTest 129/129; build-runs/fc4verify-build.log and build-runs/fc4verify-ctest.log. Owned fmt: 7 files, 0 diff lines.
- Earlier quick attempts failed a temporary type conversion and an uninitialized-view fixture; first full attempt failed clippy. All were corrected before the final full PASS; do not cite those older attempts as current results.
- Game acceptance/shadow exactness and actual merge-train enqueue are missing; no game request submitted. Raw merge entry stays wip, never ready.
- Next: resume the three partial owners, retain bounded NO40-C, measure netocc_shadow through the shared main game queue, then finish the closure audit and enqueue. No new spec questions or C ABI exports.
