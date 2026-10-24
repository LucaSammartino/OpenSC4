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
| cSC4BrushEffect | §8.3 SetTransform | `BrushEffect::set_transform`, `transform::ParentTransform::world_position` | `semantic_transform_rotates_scales_then_translates_both_owners`; open: EF19 for legacy placement decoding and native numeric order |
| cSC4BrushEffect | §8.4 shipped descriptions | `shipped::BRUSHES` | `shipped_tables`, `t01_brush_stroke_count`, `t04_one_shot_crater` |
| cSC4BrushEffect | §17.1,2,4,5 conformance | component update via manager | `ef17_01_brush_stroke`, `ef17_02_dilation`, `ef17_04_one_shot_crater`, `ef17_05_out_of_bounds` in engine/conformance/vectors |
| cSC4GameEffect | §9.1 description input | `directory::GameRecord`; `Commands::anonymous_component` | `directory_round_trip_and_locator`; `ec291_through_ec293_scrubber_aliases_crt_and_shape_overrides` |
| cSC4GameEffect | §9.2 construction, parameter 13 | `GameEffect::{new,set_parameter}` | `game_map_order_move_expiry_and_host_identity` checks uint32 data and ignored indices |
| cSC4GameEffect | §9.2 SetTransform | `GameEffect::set_transform`, `set_position` | `semantic_transform_rotates_scales_then_translates_both_owners`; open: EF19 for legacy decoding/numeric order; map geometry is the renderer's boundary |
| cSC4GameEffect | §9.2 Start, IsActive, Update, Stop | `GameEffect::{start,is_active,update,stop}` | `t07_move_to_new_cell`, `t08_pause_record`, `game_map_order_move_expiry_and_host_identity`, `game_single_does_not_remove_map_or_resume_and_absent_simulator_does_not_start` |
| cSC4GameEffect | §9.3 action order and demolition filter | `GameEffect::apply`, `demolish_rect`, `DemolishFilter::skips`, `EffectsHost::{apply_effect_map,remove_effect_map}` | `t06_game_record_order`, `demolish_filter`, `game_map_order_move_expiry_and_host_identity` |
| cSC4GameEffect | §9.4 shipped descriptions | `shipped::game_records` | `shipped_tables`, `t06_game_record_order`, `t08_pause_record` |
| cSC4GameEffect | §17.6,7,8 conformance | game update via manager | `ef17_06_game_record_order`, `ef17_07_move_to_new_cell`, `ef17_08_pause_record` in engine/conformance/vectors |
| cSC4EffectsParser | §19.1 shared dependencies | `effects_parser::{Parser,script}` | [Parser owner tables](../effects_parser/README.md) cover effects-parser §§2–14 and reader/directory/lifetime/dispatch vectors |
| cSC4EffectsParser | §§19.2–19.56 command algorithms and conformance | `effects_parser::script::commands::Commands` and child modules | [Per-section command owner tables](../effects_parser/README.md) map all published EC1–EC305; foreign calls are individual replies |
| cSC4EffectsCollection | §3 directory format | `directory::EffectDirectory` | `directory_round_trip_and_locator` |
| cSC4EffectsCollection | §19.1 → effects-parser §11.1 component identity, replacement, retention | `effects_parser::script::Directory::{lookup,register}` | `di1_through_di5_type_names_replacement_and_anonymous_identity` |
| cSC4EffectsCollection | §18.1 collection slots, §19.4 ID-resolution boundary, §19.9 copy/validation boundary | no published algorithm beyond the component directory | open: EF18 |
| EffectsParser | §6.1 ParseQueuedFiles; effects-parser §1 exclusion | `effects_parser::bootstrap::EffectsParser::parse_queued_files` | open: EF18; `unspecified_queue_and_bootstrap_do_not_claim_success` |
| EffectsBootstrap | no algorithm assigned in effects.md | `effects_parser::bootstrap::EffectsBootstrap::initialize` | open: EF18; `unspecified_queue_and_bootstrap_do_not_claim_success` |

The full §17 inventory, including manager and particle owners outside these six
owners, is recorded in the [parser README](../effects_parser/README.md#effectsmd-17-conformance-scope).
The shipped full-directory flag/LOD census (§17.13–14) is published spec data;
this lane has no complete shipped resource fixture and does not manufacture one
from the expected counts. Existing behavior vectors and scoped shipped tables
remain separate from a complete data-corpus census.

Transforms accept semantic rotation/scale/translation rather than guessing the
opaque legacy bytes. Binary32 operation order is a literal portable choice,
recorded in EF19, not a measured native numeric match. Map tokens are opaque
host identities; the core removes the previous token when count > 0 and the
current token on a started Stop, after resuming pauses. A single game update
deliberately leaves pauses and map in place, as specified.

EF17 remains reserved/open as directed by the lane brief. No EF17 answer or
duplicate is claimed. EF18 and EF19 block a claim of complete six-owner closure.
