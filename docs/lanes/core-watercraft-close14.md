| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close14 (watercraft owner integration) | Claude / `core-watercraft-close14` (engine lane) | ready: owner closure code + named vectors on branch; queue entry written | Merge train picks up `build-runs/merge-queue/core-watercraft-close14.json`. |

## Handoff

Goal: close cSC4Watercraft (coverage-map row 183). Base main 72f00fb81. Claude takeover round, 7 commits.
Added in engine/core/src/watercraft:
- interface.rs: QueryInterface/GZCLSID, +4/+8/+12/+32/+56 adjustors, Read/Write via +4, complete destruction release order, deleting destructor flags (§3.1).
- lifecycle.rs: inherited Shutdown order (`shutdown_inherited`) and `Fleet::shutdown_complete` returning its result (§3, §3.5).
- states.rs: `push_state_entered` returns base HandleEnterState result, owner entry still runs (§5.7.1).
- presentation.rs + runtime.rs `animation_tick`: in-order execution of wakes, slot 100/104/107 with request-time inherited motion (§5.1).
- Bug fixes: idle 60000 ms threshold tested regardless of +0x2CC (§5.1.2); Init zeros crash angle +0x2C4.
- Removed gap-returning `state_tick`/`state_tick_with_geometry`; ContractGap AiMotionWc4/CrashSequenceWc5 gone; `assist_pick_new_dir` added.
- Vector tests: lifecycle_, collision_vector_, route_vector_, command_vector_, boundary_vector_tests.rs plus closure_tests additions. All 199 §11 IDs now named in tests except WC1-I01..I03 and WC2-P01, which belong to cSC4WatercraftManager.
Gate on abbe7d112: qbuild verify PASS (native build, CTest 90/90) after deleting a stale sccache-built capi_smoke.obj (ABI 100 vs 102);
cargo test workspace 0 failures; cargo clippy --all-targets clean.
No SPEC-GAP markers. +0x2E4 is zeroed by Init but has no reader in the spec, so it has no field.
No ABI/plugin wiring, game runs or shadow measurements (an adapter does not exist yet).
