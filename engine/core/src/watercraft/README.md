# Watercraft core

Clean-room Rust `cSC4Watercraft` owner at `engine/core/src/watercraft/`, from
`docs/specs/watercraft.md` and its automata references. No capi/plugin wiring or
ABI changes. `Fleet` owns state and tow links; foreign services supply terrain,
resource acquisition and berth arbitration, never native owner calculations.

## Owner closure

**Implementation in progress, not owner closure.** `pending` means specified work
still to implement, not an Unsupported contract. `open` means the current spec
itself leaves the behavior unsettled. WC3–WC5 are the only open watercraft
questions from the original audit; WC1–WC2 and WC6–WC8 have answers in the spec.
Round 2 adds WC9: §6.2.2's ordered steering prose contradicts WC8-08.
The implementation follows the literal prose and does not claim that vector.

| Owner | Spec section | Code item | Test or remaining work |
|---|---|---|---|
| cSC4Watercraft | §0 | `Watercraft`, `Fleet` | Scope. |
| cSC4Watercraft | §1 | `WatercraftRecord`, runtime fields | `record_only_saves_version_and_base_and_rejects_truncation`; pending remaining inherited/runtime fields. |
| cSC4Watercraft | §2 | `DriveParameters`, `Group` | Existing absent/zero/ancestor parameter tests. |
| cSC4Watercraft | §3 | `initialize_runtime`, `shutdown` | Init factory failure and repeated-init tests; pending inherited lifecycle integration; open: WC3. |
| cSC4Watercraft | §3.1 | constructor, codec Write, `erase_travel_point`, `sign` | `wc3_l01_l05_l06_constructor_erase_and_sign`; pending type ID, deletion/release order and interface model. |
| cSC4Watercraft | §3.2 | endpoints, `maximum_extent`, `Footprint::build` | `wc5_f01_through_f04_model_footprint_builder`; pending outside-corner helper vectors. |
| cSC4Watercraft | §3.3 | `movement_possible`, docking cleanup, straight integration | Existing movement ordering tests; pending separate reported station-lot position. |
| cSC4Watercraft | §3.4 | `angular_proximity`, `quantize_heading`, `on_land`, `end_towing` | `wc2_p02_slew_and_wc3_j04_j05_angular_quantization`; pending terrain-at-cell and reentrant tow reference tests. |
| cSC4Watercraft | §4 | `push_state`, `pop_state`, `state_tick_with_geometry` | Existing state and geometry dispatch tests; pending crash dispatcher and inherited hook results; open: WC3/WC5. |
| cSC4Watercraft | §5.1 | budget, lifetime, wake and backwards substeps | Existing timer/budget tests; pending complete ordered animation tick; open: WC4 subsidiary animation/effect/model computations. |
| cSC4Watercraft | §5.1.1 | `physics.rs` | `wc4_m01_m02_*` through `wc4_m07_*`, `wc4_v01_through_v04_*`. |
| cSC4Watercraft | §5.1.2 | `count_idle_always`, budget accounting | Existing idle tests; manager population writer outside this owner. |
| cSC4Watercraft | §5.1.3 | `slew_ai_targets` | `wc2_p02_*`; pending explicit caller-removal and shore-contact gate vectors; open: WC4 target selection. |
| cSC4Watercraft | §5.1.4 | `advance_speed_tween`, `generic_seek`, wake requests | Round 2 covers tween completion and inherited-vector seeking, adjustment bypass, overshoot and zero-time division. Pending full state/flag dispatch and stop-update ordering; open: WC4 subsidiary calculations. |
| cSC4Watercraft | §5.2 | `candidate_directions`, `fill_route_points`, `backtrack_travel` | Neighbor/ferry fill to ten and WC7-R02 survivor-mask rule. Pending wandering search orchestration; open: WC4 AI target selection. |
| cSC4Watercraft | §5.3 | `dock_requests`, `docking_tick`, cleanup | Existing berth/queue and final approach tests; pending distinct lot/berth coordinates. |
| cSC4Watercraft | §5.3.1 | `docking_tick` final approach | Strict pre-motion completion, unclamped time and antiparallel sign implemented; pending all named edge vectors. |
| cSC4Watercraft | §5.3.2 | Docking entry height flatten/land continuation | Far branch remains open: WC4 AI target selection; pending far-call ordering scaffold. |
| cSC4Watercraft | §5.4 | station and neighbor setup | Existing station tests; pending nullable neighbor occupant, terrain-water skip point. |
| cSC4Watercraft | §5.4.1 | `RouteReader`, quantization, reservation cleanup | `wc7_r03_through_r06_*`; pending full route-fill/wrap vectors and null occupant behavior. |
| cSC4Watercraft | §5.5 | `undocking_tick` | Updated motion/completion test; pending named blocked, on-land, missing-occupant vectors. |
| cSC4Watercraft | §5.6 | movement gates | Existing flag/bounds/crash-state ordering test; pending inherited FindState order audit. |
| cSC4Watercraft | §5.7 | `detect_collision` | Outside removal precedes boat/shore scans; boat crash returns early, bumps continue to shore. Focused outside/shore tests; pending comprehensive composed boat/shore vectors. |
| cSC4Watercraft | §5.7.0 | `Footprint::overlaps`, `find_overlapping_boat`, `respond_to_boat_collision` | SAT contact and size-tie/bump/player-victim tests; pending all scan/size vectors and two-slot AEMS event observation. |
| cSC4Watercraft | §5.7.1 | Crash state entry/timer prefix | Pending specified dispatcher, double dt, effects and tween reset; open: WC5 crash prerequisites. |
| cSC4Watercraft | §5.7.2 | `bump_rollback`, `sinking_tick`, `heel_angle_step` | WC5-S01–S07 scalar/height vectors and WC5-K01–K04 sinking orientation/heading feedback. Heel helper is scalar-only; open: WC5 heel/contact composition. |
| cSC4Watercraft | §5.7.3 | `Footprint::build` | `wc5_f01_through_f04_*`; missing geometry retains caller's explicit scratch validity/axis. |
| cSC4Watercraft | §5.7.4 | `inanimate_collision` | `wc5_i01_through_i05_*`, all 32 table cells and scan order. |
| cSC4Watercraft | §6.1 | coast, tow release, movement | Pending complete drive tick; open: WC4 inherited player-drive computations. |
| cSC4Watercraft | §6.2–§6.2.3 | Float `DriveCommand`, nonlinear/inverse curves, `CommandChannel`, `SpeedTween` | WC8 speed/steering/toggle and WC6 fire/transform/render/tween/cooldown vectors. WC8-08 disputed in WC9; nullable missing-group input unrepresentable (`Arc<Group>` required). Pending tow-effect transform/refresh and complete drive-tick integration. |
| cSC4Watercraft | §6.3 | `following_tick`, tow hooks | `following_uses_endpoint_gap_and_slack_does_not_publish`; pending named endpoint/negative-time vectors. |
| cSC4Watercraft | §7 | `do_message`, insertion-order fleet scan | First geometric-match selection; pending explicit rejection-of-first-candidate and radius equality vectors. |
| cSC4Watercraft | §8 | State/idle fallthrough | Existing idle test; crash continuation pending. |
| cSC4Watercraft | §9 | `debug_extra_text` | Existing formatting test; pending shared borrowed-string identity/lifetime API. |
| cSC4Watercraft | §10 | Contract boundary | open: WC3 inherited/tuning, WC4 AI/subsidiary updates, WC5 heel/contact and new WC9 steering contradiction. |
| cSC4Watercraft | §11 | `tests.rs`, `physics_tests.rs`, `collision_tests.rs`, `round2_tests.rs` | In-progress vector coverage listed above, not all named vectors implemented. |

The record test `record_same_input_round_trip_stabilizes_inherited_loaded_status`
checks equality of a decoded record and its same-input second round trip, and byte
stability after the inherited loaded-status normalization.

An in-game shadow is feasible after an adapter exists: capture tuning, positions,
headings, dt and model/terrain geometry, then compare proposed runtime and visual
transforms, route coordinates, state transitions and collision decisions. The full
owner shadow requires pending work and resolution of WC3–WC5. No game run was
requested for this unwired core module.
