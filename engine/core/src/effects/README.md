# Effects owner closure

Pure Rust component and parser owner work from effects.md. `effects_parser` owns
the token-record command boundary; it does not claim a lexer, file loader or
native exception ABI. Renderer operations are individual host calls. No plugin,
C ABI or game replacement is introduced.

## Owner closure

| Owner | Spec section | Code item | Test or open question |
|---|---|---|---|
| cSC4BrushEffect | §8.1 description input | `directory::BrushRecord`; `effects_parser::script::commands::Commands::anonymous_component` | `directory_round_trip_and_locator`; `ec288_through_ec290_sound_brush_maps_and_stored_length` |
| cSC4BrushEffect | §8.2 creation | `BrushEffect::new` | `brush_draws_twice_even_for_constant_ranges_and_start_resets_stroke` |
| cSC4BrushEffect | §8.3 Start, IsActive, Update, Stop, Apply | `BrushEffect::{start,is_active,update,stop,apply}` | `t01_brush_stroke_count`, `t04_one_shot_crater`, `t05_out_of_bounds`, `volcano_finishes_in_the_draw_pass_or_the_hook` (includes repeated Stop) |
| cSC4BrushEffect | §§3.1,8.3 SetTransform | `BrushEffect::{set_transform,set_placement}`, `transform::Placement::world_position` | `ef19_3_both_component_owners_use_rows_and_translation_only`; `ef19_4_5_6_flags_precedence_cancellation_and_zero_sign`; `extended_exponent_and_double_precision_store_boundaries` |
| cSC4BrushEffect | §8.4 shipped descriptions | `shipped::BRUSHES` | `shipped_tables`, `t01_brush_stroke_count`, `t04_one_shot_crater` |
| cSC4BrushEffect | §17.1,2,4,5 conformance | component update via manager | `ef17_01_brush_stroke`, `ef17_02_dilation`, `ef17_04_one_shot_crater`, `ef17_05_out_of_bounds` in engine/conformance/vectors |
| cSC4GameEffect | §9.1 description input | `directory::GameRecord`; `Commands::anonymous_component` | `directory_round_trip_and_locator`; `ec291_through_ec293_scrubber_aliases_crt_and_shape_overrides` |
| cSC4GameEffect | §9.2 construction, parameter 13 | `GameEffect::{new,set_parameter}` | `game_map_order_move_expiry_and_host_identity` checks uint32 data and ignored indices |
| cSC4GameEffect | §§3.1,9.2 SetTransform | `GameEffect::{set_transform,set_placement,set_position}` | `ef19_3_both_component_owners_use_rows_and_translation_only`; `ef19_4_5_6_flags_precedence_cancellation_and_zero_sign`; `extended_exponent_and_double_precision_store_boundaries`; map geometry remains a renderer primitive |
| cSC4GameEffect | §9.2 Start, IsActive, Update, Stop | `GameEffect::{start,is_active,update,stop}` | `t07_move_to_new_cell`, `t08_pause_record`, `game_map_order_move_expiry_and_host_identity`, `game_single_does_not_remove_map_or_resume_and_absent_simulator_does_not_start` |
| cSC4GameEffect | §9.3 action order and demolition filter | `GameEffect::apply`, `demolish_rect`, `DemolishFilter::skips`, `EffectsHost::{apply_effect_map,remove_effect_map}` | `t06_game_record_order`, `demolish_filter`, `game_map_order_move_expiry_and_host_identity` |
| cSC4GameEffect | §9.4 shipped descriptions | `shipped::game_records` | `shipped_tables`, `t06_game_record_order`, `t08_pause_record` |
| cSC4GameEffect | §17.6,7,8 conformance | game update via manager | `ef17_06_game_record_order`, `ef17_07_move_to_new_cell`, `ef17_08_pause_record` in engine/conformance/vectors |
| cSC4EffectsParser | §19.1 shared dependencies | `effects_parser::{Parser,script}` | [Parser owner tables](../effects_parser/README.md) cover effects-parser §§2–14 and reader/directory/lifetime/dispatch vectors |
| cSC4EffectsParser | §§19.2–19.56 command algorithms and conformance | `effects_parser::script::commands::Commands` and child modules | [Per-section command owner tables](../effects_parser/README.md) map all published EC1–EC305; foreign calls are individual replies |
| cSC4EffectsCollection | §3 directory format | `directory::EffectDirectory` | `directory_round_trip_and_locator` |
| cSC4EffectsCollection | §19.1 → effects-parser §11.1 component identity, replacement, retention | `effects_parser::script::Directory::{lookup,register}` | `di1_through_di5_type_names_replacement_and_anonymous_identity` |
| cSC4EffectsCollection | §§18.1,18.6 Begin/End, camera and reload | `collection::Collection::{begin,end,set_camera}` | `ef18_9_10_ids_duplicate_triggers_and_empty_reload` |
| cSC4EffectsCollection | §18.6 visual publication/copies and §19.9 copy/validation boundary | `Collection::{add_description,get_description}` | `ef18_7_8_11_copies_stable_slots_fixups_only_at_publication`; `collection_parameter_binding_precedence_and_each_component_directory` |
| cSC4EffectsCollection | §18.6 IDs/messages and §19.4 ID-resolution boundary | `Collection::{add_binding,add_trigger,end}` | `ef18_9_10_ids_duplicate_triggers_and_empty_reload` |
| EffectsParser | §6.2 parser installation, retained queue and watchers | `effects_parser::bootstrap::EffectsParser::{install,append,num_input_files,input_file_path,set_files_modified,files_modified}` | `ef18_1_retained_queue_repeated_success_watchers_and_parser_references` |
| EffectsParser | §§6.1,6.2 synchronous ParseQueuedFiles and error reporting | `EffectsParser::{parse_all,parse_queued_files}` | `ef18_1_retained_queue_repeated_success_watchers_and_parser_references`; `ef18_2_6_failure_retains_publications_and_not_ready_reports_success`; `ef18_12_optional_open_exception_and_other_optional_exceptions_succeed` |
| EffectsBootstrap | §6.2 packed selection, readers, file loading and cleanup | `effects_parser::bootstrap::EffectsBootstrap::initialize`; `EffectsParser::shutdown` | `ef18_3_packed_reader_order_skips_all_instance_one_keys`; `ef18_4_unpacked_failure_continues_progress_detach_and_end`; `ef18_5_absent_resources_skips_packed_files_and_options_do_not_trim`; `packed_existing_main_retains_parser_and_resource_failures_do_not_abort`; queue reference/shutdown test above |
| cSC4EffectsResource | §3 variable-length visual/component records, including 56-byte placement framing | `directory::{read_effect,read_component,write_effect}` | `section3_variable_component_stride_preserves_all_placement_bytes`: independent binary fixture, two name lengths, all required prefixes rejected when truncated |
| cSC4EffectsResource | §3.1 legacy placement load/write semantics | `transform::Placement::{load,decode,encode}` | `ef19_1_2_decode_revision_flags_and_float_bits` |
| cSC4EffectsResource | §3 resource tables, name map and section location | `EffectDirectory::{parse,parse_at,to_bytes}` | `directory_round_trip_and_locator`; unknown render sections are located, not decoded |
| cSC4BrushDescription | §8.1 binary fields; §§19.52,19.55 description commands | `directory::{BrushRecord,read_brush,write_brush}`; `Commands::anonymous_component` | `directory_round_trip_and_locator`; `ec288_through_ec290_sound_brush_maps_and_stored_length` |
| cSC4ScrubberDescription | §9.1 binary fields/defaults; §§19.52,19.55 description commands | `directory::{GameRecord,read_game,write_game}`; `Commands::anonymous_component` | `directory_round_trip_and_locator`; `ec291_through_ec293_scrubber_aliases_crt_and_shape_overrides` |

## Scope and conformance

EF18 and EF19 are implemented at the Rust owner boundary. Queue and bootstrap
functions are manager operations, with individual foreign resource/file/message
calls; collection methods own the publication copies and derived lookup state.
No activation, transaction rollback or independent bootstrap COM object is added.
`legacy_component_offsets_survive_creation_and_parent_moves` checks that resource
placement decoding reaches brush/game owners through manager creation and moves.

The full §17 inventory, including manager and particle owners outside these six
owners, is recorded in the [parser README](../effects_parser/README.md#effectsmd-17-conformance-scope).
The shipped full-directory flag/LOD census (§17.13–14) is published spec data;
this lane has no complete shipped resource fixture and does not manufacture one
from the expected counts. Existing behavior vectors and scoped shipped tables
remain separate from a complete data-corpus census.

`identity_parent_uses_local_translation_directly` additionally checks the §8.3
identity-parent fast path preserves the original translation bits, including
negative zero.

`Placement` carries explicit flags and revision, decodes the little-endian
resource transform and implements the specified z/x/y arithmetic order and
binary32 store boundaries for 24- and 53-bit precision. Semantic convenience
inputs remain available; use explicit placements for flag-sensitive behavior.
Framing serialization deliberately preserves opaque original bytes, while
semantic `Placement::encode` zero-extends only the loaded revision byte.
No game shadow or native renderer equivalence is claimed. Map tokens are opaque
host identities; the core removes the previous token when count > 0 and the
current token on a started Stop, after resuming pauses. A single game update
deliberately leaves pauses and map in place, as specified.

The existing manager/particle and shipped-corpus scope above is unchanged.
No new spec question or Unsupported boundary is introduced by EF18/EF19.
