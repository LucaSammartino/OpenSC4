# Construction occupant

`lot_records::construction` owns cSC4ConstructionOccupant. It reuses
`automata::OccupantBase`, `ConstructionRecord`, the shared stream codecs,
`RzRandom`, and `CrtRand`. No second occupant base or plugin/ABI implementation
is introduced. Foreign lot, city, terrain, prop/automata manager, renderer and
model services are `ConstructionHost`; resource lookups are `ConstructionSource`.
The caller supplies one process-owned `ConstructionStatics` (load once after
base Init); its file RNG and the process CRT RNG are explicit for determinism.

## Owner closure

Only sections assigning this owner behavior are listed; other classes in these
two specs stay foreign. `core:` names tests in `lot_records/tests_construction.rs`;
`conformance:` names executable tests in `engine/conformance/tests/construction_owner.rs`.
Open rows are deliberately not claims of full visual closure.

| Spec section / owner | Code item | Coverage or open question |
|---|---|---|
| lot-records §1 identity / construction | `CLSID_CONSTRUCTION_OCCUPANT`, `ConstructionRecord` | core: `record_layout_and_round_trip` |
| lot-records §2.1 encodings / construction | shared stream/base codecs | core: `bool_fields_read_nonzero_and_write_one` |
| lot-records §2.2 versions / construction | `ConstructionRecord::accepts/read/write` | core: `version_rules`, `v4_legacy_transform_record` |
| lot-records §2.3 base / construction | shared `OccupantBase` | core: `record_layout_and_round_trip` |
| lot-records §2.4 Init / construction | `ConstructionOccupant::init` | core: `init_adds_occupant_group_once` |
| lot-records §9 record / construction | `ConstructionRecord`, `CrewEntry` | core: `record_layout_and_round_trip`, `xform_read_rules_no15` |
| lot-records §9.1 live fields / construction | `ConstructionScalars`, `rebuild_model`, `bounding_box` | core: `loaded_construction_resumes_schedule`; conformance: `world_bounds_and_visual_messages_and_uninitialised_shutdown` |
| lot-records §9.2 lifecycle / construction | `init/set_lot/do_message/refresh/shutdown`, visual message accessors | core: `set_lot_order_and_completion_order`, `refresh_after_load_recreates_crane_once`; open: LD36, LD38 (phase interpolation, render preparation IDs) |
| lot-records §13.1 serializable / construction | existing `RecordObject<ConstructionRecord>` | core: `record_layout_and_round_trip` |
| lot-records §13.2 called interfaces / construction | `ConstructionHost` boundary | conformance: `test47_completion_removes_every_manager_entry_once` |
| lot-records §14 load resolution / construction | `read_fields`, typed reference reads | core: `record_layout_and_round_trip`, `live_refs_released_by_set_lot_read_and_shutdown` |
| lot-records §15.1 creation / construction | existing class registry factory; foreign lot developer creates and inserts | core: `record_layout_and_round_trip`; no plugin wiring requested |
| lot-records §15.2 coupling / construction | shared base, record-held references and host services | core: `loaded_construction_resumes_schedule` |
| lot-records §16 quirks / construction | live-vs-loaded crane flags, future version reads, scalar bytes | core: `version_rules`, `refresh_after_load_recreates_crane_once` |
| lot-records §17.1 corpus validation / construction | no game save corpus accessed | synthetic save coverage in core: `record_layout_and_round_trip` |
| lot-records §17.2 round trips / construction | no construction-specific corpus vector prescribed | core: `record_layout_and_round_trip`, `v4_legacy_transform_record` |
| lot-records §17.3 behavior / construction | no construction-specific numbered test in this section | core: `loaded_construction_resumes_schedule`, `refresh_after_load_recreates_crane_once` |
| lot-records §17.4 native boundaries / construction | foreign class-factory handoff | no native seam requested; existing core factory retained |
| lot-developer §6.3 handoff / construction | `set_lot`, `complete`; caller owns insertion | core: `set_lot_order_and_completion_order` |
| lot-developer §6.7 owner overview / construction | `ConstructionOccupant` | core: `test21_construction_schedule` |
| lot-developer §6.7.1 identity/Init/messages / construction | `init/do_message/visibility_test/model_available` | core: `init_adds_occupant_group_once`, `animation_tick_message_divides_by_1000`; conformance: visual message test; open: LD38 |
| lot-developer §6.7.2 statics / construction | `ConstructionTuning::from_properties`, `ConstructionStatics::load` | core: `tuning_shipped_and_defaults`; conformance: `static_tables_filter_missing_profiles_and_reseed_plain_time` |
| lot-developer §6.7.3 geometry / construction | `rebuild_model`, widened saved world box | core: `test20_construction_timing`; conformance: world bounds test; open: LD35 (mesh only) |
| lot-developer §6.7.4 SetLot / construction | `set_lot`, `timing`, placement helpers | core: `set_lot_order_and_completion_order`, `set_lot_crane_initial_values`; open: LD35, LD37, LD38 (visual details) |
| lot-developer §6.7.5 daily phases / construction | `do_daily_sim`, `complete`, `set_building_completion` | core: `test21_construction_schedule`, `set_building_completion_posts_only_on_change`; conformance: test47 |
| lot-developer §6.7.6 tick / construction | `animation_tick`, `animate_crane`, `crane_duration`, `swing_limits` | core: `test25_crane_pause`, `crane_duration_uses_tunable_t_and_k`; open: LD36 (phase interpolation) |
| lot-developer §6.7.7 random placement / construction | `ground_props/roof_props/crew/place_prop/create_worker`, `lot_texture/scatter_sparks` | core: `test23_lot_texture`, `test24_spark_scatter`, `spark_counts`; conformance: test45, test47; literal open details: LD37, LD38 |
| lot-developer §6.7.8 tick manager / construction | priority `0x80`, unsigned µs/1000 conversion | core: `animation_tick_message_divides_by_1000`; manager scheduling stays foreign |
| lot-developer §6.7.9 queued messages / construction | host posts from `set_lot/do_daily_sim/complete/shutdown` | core: `set_lot_order_and_completion_order` |
| lot-developer §6.7.10 saved state / construction | `ConstructionRecord`, world box rebuild | core: `loaded_construction_resumes_schedule`; conformance: world bounds test |
| lot-developer §6.7.11 lot division / construction | `divide_lot` | conformance: `test44_divide_lot_and_nan_and_inverted` |
| lot-developer §6.7.12 cranes / construction | `CraneSize`, `outer_crane`, `inner_crane_choice`, `remove_rectangle`, `swing_limits`, `crane_model_angle` | conformance: test45, test46; open: LD35 (inner pose/CreateCrane mesh), LD38 (saved y/cut clipping) |
| lot-developer §6.7.13 prop tables / construction | `ConstructionStatics::load`, `GROUND_TYPES/ROOF_TYPES` | conformance: static tables test |
| lot-developer §6.7.14 cleanup / construction | `shutdown`, roof-prop drain at removal | conformance: `test47_completion_removes_every_manager_entry_once`; core: `live_refs_released_by_set_lot_read_and_shutdown` |
| lot-developer §6.7.15 explicitly open visuals / construction | `Unsupported` variants | open: LD35; detail setting passed as an opaque integer |
| lot-developer §10/§12 corrections / construction | isolated file RNG, table iterators and explicit seed | core: `test24_spark_scatter`; conformance: static tables/test45 |
| lot-developer §14 dependencies / construction | `ConstructionHost/ConstructionSource` | conformance: test47 |
| lot-developer §16 tests 20,21,23–25 / construction | geometry/timing/schedule/texture/sparks/crane functions | core tests of matching numbers; conformance: construction owner tests |
| lot-developer §16 tests 44–47 / construction | division, swing, cut, manager cleanup | conformance: `test44_*`, `test45_*`, `test46_*`, `test47_*` |

## Limits

LD35–LD38 are in `docs/specs/questions/lot-developer.md`. Exact visual mesh,
phase interpolation, inner pose, and unharvested render/effect parameters remain
open; numerical timing and manager cleanup do not depend on them. Literal
choices for unspecified anchors, crew flags, and cut clipping are commented
at the code site. `CreateCrane` and effect/resource allocation are foreign model
service operations, never callbacks that decide crane fit or prop placement.
No real-game run is applicable without plugin wiring, which this task forbids.
