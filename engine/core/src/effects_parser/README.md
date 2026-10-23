# Effects description parser

This module extends the existing `effects` component representation. It models
owner decisions using foreign primitive replies, not a text lexer or native
command interpreter. Renderer, sound, resource, conversion and generic command
library services remain foreign. No plugin or C ABI integration is provided.

Command owners are implemented through effects §19.56/EC305 and effects-parser
§14/NQ17. EF15, EF16 and EFP7 are answered and implemented. The registration
inventory and owner tables below cover the published command boundary. Known
class names are used only where published. Conversion spelling, lexing, native
exception delivery and rotation transcendental accuracy remain foreign services.

## Owner closure

`root_registration_order_literals_descriptors_hooks_and_construction` (abbreviated
**root test** below) independently checks all 21 literal/descriptor/hook tuples,
their order, allocation size and block construction. All test items below are in
`tests.rs` or its included `extra_tests.rs` unless explicitly qualified.

| Command owner (descriptor; literal) | Spec section | Code item and test | Execution closure |
|---|---|---|---|
| `00A9F85C`; `renderprop` | effects-parser §6.1 | `ROOT_COMMANDS[0]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F86C`; `textureID` | effects-parser §6.1 | `ROOT_COMMANDS[1]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F87C`; `modelID` | effects-parser §6.1 | `ROOT_COMMANDS[2]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F88C`; `soundID` | effects-parser §6.1 | `ROOT_COMMANDS[3]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F89C`; `brushID` | effects-parser §6.1 | `ROOT_COMMANDS[4]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F8AC`; `effectID` | effects-parser §6.1 | `ROOT_COMMANDS[5]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F8BC`; `effectGroup` | effects-parser §6.1 | `ROOT_COMMANDS[6]`, `register_commands`; root test | registration covered; execution table below |
| `00A9FA08`; `instance` | effects-parser §6.1 | `ROOT_COMMANDS[7]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F8D8`; `testEffect` | effects-parser §6.1 | `ROOT_COMMANDS[8]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F8E8`; `messageTrigger` | effects-parser §6.1 | `ROOT_COMMANDS[9]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F8F8`; `camera` | effects-parser §6.1 | `ROOT_COMMANDS[10]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F908`; `setPriority` | effects-parser §6.1 | `ROOT_COMMANDS[11]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F918`; `effectsResource` | effects-parser §6.1 | `ROOT_COMMANDS[12]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F934`; `loadResource` | effects-parser §6.1 | `ROOT_COMMANDS[13]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F944`; `effect` | effects-parser §6.1 | `ROOT_COMMANDS[14]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F960`; `particles` | effects-parser §6.1 | `ROOT_COMMANDS[15]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F97C`; `dynamicParticle` | effects-parser §6.1 | `ROOT_COMMANDS[16]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F998`; `decal` | effects-parser §6.1 | `ROOT_COMMANDS[17]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F9B4`; `shake` | effects-parser §6.1 | `ROOT_COMMANDS[18]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F9D0`; `light` | effects-parser §6.1 | `ROOT_COMMANDS[19]`, `register_commands`; root test | registration covered; execution table below |
| `00A9F9EC`; `sequence` | effects-parser §6.1 | `ROOT_COMMANDS[20]`, `register_commands`; root test | registration covered; execution table below |
| `00A9FDFC`; `sequenceEffect` | effects-parser §6.2 registration | `SEQUENCE_EFFECT`, `register_sequence_effect`; `nested_registration_and_allocation_boundary_are_separate` | published association covered |
| `00A9FDFC`; `sequenceEffect` | effects-parser §6.2 execution | `Parser::sequence_effect`; `r1_sequence_named_repeat_append`, `r2_options_override_initial_clear`, `unsupported_command_and_validation_keep_precise_prefix`, `sequence_command_reads_selection_before_null_manager_append_short_circuit`, `sequence_command_uses_only_first_selection_and_preserves_name_case` | normal primitive-input domain covered; logical failure prefix covered; native exception ABI stays foreign |
| Nested command owners | effects §§19.53–19.56 | `script::commands::components`; owner rows below | EC277–EC305 covered |

### Shared parser operations used by commands

| Owner / spec section | Code item | Tests / remaining open domain |
|---|---|---|
| Parser option selection; effects-parser §2 | `parse_options`, `OptionPrimitives` | `p1_absent_retains_asymmetric_fields`, `p7_duplicate_alternatives_are_distinct_foreign_fixtures`; concrete reader and rotation instruction backend covered below |
| Parser nonrotation options; effects-parser §§3–4 | `parse_options`, `scalar`, `probability` | `p1_*` through `p8_*`, `scale_compares_original_after_rounded_store`, `forbidden_lod_and_probability_stop_at_error_boundary`; numeric qualification in §14 rows below |
| Parser named append; effects-parser §5.1 | `Parser::add_named`, `Parser::finish` | `r1_sequence_named_repeat_append`, `r2_options_override_initial_clear`, `r5_null_manager_short_circuits_both_paths`, `r6_capacity_branch_is_after_mutation`, `named_rereads_shared_first_index_after_validation`; directory identity covered below |
| Parser anonymous append; effects-parser §5.2 | `Parser::add_anonymous` | `r3_anonymous_sentinel_is_success_and_full_type_retained`, `r4_anonymous_raw_index_has_no_local_counter`, `r5_null_manager_short_circuits_both_paths`; directory identity covered below |
| Shared record constructor; effects-parser §5.3 | `Record::shared_constructor_projection` | `shared_constructor_projection_is_not_direct_option_initialization` |
| Parser manager acquisition; effects-parser §5.4 | `Parser::set_manager` | `m1_distinct_manager_reference_order` through `m4_nonnull_manager_null_qi_clears_k`; reentry excluded by spec |
| Parser interface identity; effects-parser §5.5 | `query_interface`, `QueryIdentities` | `q1_logical_adjusted_identity_and_nullout`, `query_fallback_is_raw_and_missing_adjustment_unsupported`; explicit logical lifetime covered below |
| Command library; effects-parser §6.1 | `register_one`, `register_commands` | root test, `nested_registration_and_allocation_boundary_are_separate`; concrete dispatch/ownership covered below |
| Parser trace ledgers; effects-parser §7 | operations above | `r1_*`–`r6_*`, `m1_*`–`m4_*`, `q1_*`; stipulated foreign fixtures, not native golden executions |
| Coverage boundary; effects-parser §8 | module-level documentation, typed `Unsupported` | error-prefix tests retain earlier writes; whole parser remains partial as specified |

## Owner closure: settled command follow-up

This table records executable owner work from commands5b and commands5c.
Particle execution (§§19.18–26,19.39–51), decal children (§§19.35–38), and
visual-effect/group dictionaries and execution (§§19.52–56) are covered below.
New nested float primitives accept canonical raw extended replies; legacy f64
and binary32 replies are lifted exactly. Numeric control is explicit PC24/53/64
and nearest/down/up/zero, with all exceptions masked. The historical
`NearestAtLeast53` fixture policy explicitly selects PC53/nearest; no live game
control word is inferred. Warp sine/cosine remain separate primitive calls.
Ordinary allocation and valid selected-index domains apply;
an unchecked empty selection is Unsupported rather than an invented parser error.

| Owner / spec section | Code item | Tests |
|---|---|---|
| Reader, effects-parser §9 | script::find_switch, find_positional, check_switches | sw1_through_sw8_reader_records_and_failure_prefix |
| Error prefix, effects-parser §10 | script::Error; Commands fallible operations | ec31_through_ec42_priorities_visual_reset_flags_and_failure_prefixes; ec47_ec48_ec49_ec50_preview_failure_gates_and_late_error_prefix |
| Directory, effects-parser §11.1 | script::Directory | di1_through_di5_type_names_replacement_and_anonymous_identity |
| Parser lifetime, effects-parser §11.2 | script::Lifetime | lt1_lt2_lt3_final_release_order_and_aliased_held_slots |
| Registrar, effects-parser §12.1 | script::Dictionary::root, child, destroy_root, destroy_block | dr1_dr2_dr3_duplicate_ownership_and_case |
| Dispatcher, effects-parser §12.2 | script::Dictionary::dispatch | dr4_dr5_dr6_nested_precedence_throwing_end_and_switch_asymmetry |
| cRenderPropCommand, effects §19.2 | Commands::render_property | ec1_ec2_ec3_ec4_renderer_precedence_and_arity |
| cEffectGroupIDCommand, effects §19.2 | Commands::enter_group; return-only exit | ec5_through_ec15_identifier_order_and_exception_prefixes |
| textureID descriptor 00A9F86C, effects §19.4 | Commands::declare_id(0) | ec5_through_ec15_identifier_order_and_exception_prefixes |
| modelID descriptor 00A9F87C, effects §19.4 | Commands::declare_id(1) | ec5_through_ec15_identifier_order_and_exception_prefixes |
| soundID descriptor 00A9F88C, effects §19.4 | Commands::declare_id(2) | ec5_through_ec15_identifier_order_and_exception_prefixes |
| brushID descriptor 00A9F89C, effects §19.4 | Commands::declare_id(3) | ec5_through_ec15_identifier_order_and_exception_prefixes |
| effectID descriptor 00A9F8AC, effects §19.4 | Commands::effect_id(false) | ec5_through_ec15_identifier_order_and_exception_prefixes |
| instance descriptor 00A9FA08, effects §19.4 | Commands::effect_id(true) | ec5_through_ec15_identifier_order_and_exception_prefixes |
| messageTrigger descriptor 00A9F8E8, effects §19.4 | Commands::message | ec5_through_ec15_identifier_order_and_exception_prefixes |
| cEffectsResourceCommand, effects §19.5 | Commands::enter_resource, exit_resource | ec16_through_ec24_resource_gates_order_and_retained_error_state |
| cLoadResourceCommand, effects §19.5 | Commands::load_resource | ec16_through_ec24_resource_gates_order_and_retained_error_state |
| cCameraParamsCommand, effects §19.7 | Commands::camera | ec25_through_ec30_camera_expansion_switch_order_span_and_null_collection |
| setPriority descriptor 00A9F908, effects §19.7 | Commands::priority | ec31_through_ec42_priorities_visual_reset_flags_and_failure_prefixes |
| cGroupEffectCommand, effects §19.9 | Commands::enter_visual, exit_visual; existing effects::EffectDescription payload | ec31_through_ec42_priorities_visual_reset_flags_and_failure_prefixes |
| cTestEffectCommand, effects §19.11 | Commands::preview | ec43_ec44_ec45_ec46_preview_defaults_positions_and_ordered_setters; ec47_ec48_ec49_ec50_preview_failure_gates_and_late_error_prefix |
| Rotation option owner, effects-parser §13 | parse_options; rotation::apply, compose; Arithmetic backend | ro1_each_zero_axis_preserves_matrix_and_wraps_marks; ro2_all_keys_have_fixed_axis_conversion_order_and_nine_marks; ro3_ro4_compound_failures_retain_only_completed_axes; ro5_ro6_arithmetic_stage_sine_store_asymmetries; Unsupported without a qualified backend, explicitly permitted by §13 |
| Scale/probability numeric owner, effects-parser §14 | `numeric::Extended`, `Control`, `probability`; `parse_options` raw scale comparison | `numeric::tests::nq1_through_nq8_raw_scale_store_and_original_comparison`, `nq9_through_nq17_probability_intermediates_and_store`, `masked_signaling_nan_canonical_domain_and_extreme_exponents`, `arithmetic_rounding_is_distinct_from_store_and_preserves_distant_sticky_bits`; parser `scale_compares_original_after_rounded_store`, `raw_binary32_and_exact_probability_domain` |
| Numeric conformance, effects-parser §14.1 | same numeric operations, exposed m/a/store boundaries | NQ1–NQ17 in the two `nq*` tests; every raw reply checked independently before expected intermediate/store bits |
| cParticleEffectCommand, effects §19.13 | Commands::enter_definition(Particles); Description::reset | ec51_ec52_ec53_ec54_ec55_definition_defaults_and_inherit_failure; ec56_ec57_particle_retained_word_indeterminate_word_and_scratch_tail |
| cParticleEffectCommand, effects §19.14 | Commands::exit_definition(Particles) | ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication |
| cParticleEffectCommand, effects §19.17 | nested::PARTICLES, nested::register | ec69_ec70_ec71_particle_registration_order_and_distinct_alias_allocations |
| cDynamicParticleEffectCommand, effects §19.13 | Commands::enter_definition(DynamicParticle) | ec51_ec52_ec53_ec54_ec55_definition_defaults_and_inherit_failure |
| cDynamicParticleEffectCommand, effects §19.14 | Commands::exit_definition(DynamicParticle) | ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication |
| cDynamicParticleEffectCommand, effects §19.27 registration | nested::DYNAMIC_PARTICLE | ec120_ec131_ec141_dynamic_sequence_shake_light_registration |
| cDecalEffectCommand, effects §19.13 | Commands::enter_definition(Decal) | ec51_ec52_ec53_ec54_ec55_definition_defaults_and_inherit_failure |
| cDecalEffectCommand, effects §19.14 | Commands::exit_definition(Decal) | ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication |
| cShakeEffectCommand, effects §19.13 | Commands::enter_definition(Shake) | ec51_ec52_ec53_ec54_ec55_definition_defaults_and_inherit_failure |
| cShakeEffectCommand, effects §19.14 | Commands::exit_definition(Shake) | ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication |
| cShakeEffectCommand, effects §19.31 | nested::SHAKE | ec120_ec131_ec141_dynamic_sequence_shake_light_registration |
| cLightEffectCommand, effects §19.13 | Commands::enter_definition(Light) | ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication |
| cLightEffectCommand, effects §19.14 | Commands::exit_definition(Light) | ec58_ec59_ec60_ec61_light_exit_fallbacks_and_definition_publication |
| cLightEffectCommand, effects §19.31 | nested::LIGHT | ec120_ec131_ec141_dynamic_sequence_shake_light_registration |
| cSequenceEffectCommand, effects §§19.13–14 | Commands::enter_definition(Sequence), exit_definition(Sequence) | ec62_ec63_ec64_sequence_reference_replacement_exit_and_throw |
| cParticleEffectBaseCommand, effects §19.16 | Commands::effect_base | ec65_ec66_ec67_ec68_effect_base_guard_copy_empty_and_post_check |
| dynamic effectBase descriptor 00A9FBB8, effects §19.27 | Commands::dynamic_base | ec121_through_ec130_dynamic_guard_stores_and_model_transaction_boundary |
| dynamic mass descriptor 00A9FB98, effects §19.27 | Commands::dynamic_mass | ec121_through_ec130_dynamic_guard_stores_and_model_transaction_boundary |
| cDynamicParticleFrictionCommand, effects §19.27 | Commands::dynamic_friction | ec121_through_ec130_dynamic_guard_stores_and_model_transaction_boundary |
| cDynamicParticleModelCommand, effects §19.27 | Commands::dynamic_model_selected; full EAX and indices are separate primitive inputs | ec121_through_ec130_dynamic_guard_stores_and_model_transaction_boundary |
| sequence wait descriptor 00A9FCC8, effects §19.29 | nested::SEQUENCE; Commands::sequence_element(false) | ec120_ec131_ec141_dynamic_sequence_shake_light_registration; ec132_through_ec140_sequence_endpoint_appends_and_failure_atomicity |
| sequence play descriptor 00A9FCB8, effects §19.29 | nested::SEQUENCE; Commands::sequence_element(true) | ec132_through_ec140_sequence_endpoint_appends_and_failure_atomicity |
| cShakeLengthCommand, effects §19.32 | Commands::shake_light_length(false) | ec142_through_ec152_shake_scalar_nan_clamps_and_light_cross_write |
| shakeAspect descriptor 00A9FC48, effects §19.32 | Commands::shake_aspect | ec142_through_ec152_shake_scalar_nan_clamps_and_light_cross_write |
| shake table descriptor 00A9FC38, effects §19.32 | Commands::shake_table | ec142_through_ec152_shake_scalar_nan_clamps_and_light_cross_write |
| light length descriptor 00A9FC98, effects §19.32 | Commands::shake_light_length(true) | ec142_through_ec152_shake_scalar_nan_clamps_and_light_cross_write |
| shake amplitude descriptor 00A9FC68, effects §19.33 | Commands::shake_light_curve(Shake,8) | ec153_through_ec160_curves_color_aliases_and_unscaled_failure_prefix |
| shake frequency descriptor 00A9FC58, effects §19.33 | Commands::shake_light_curve(Shake,0x14) | ec153_through_ec160_curves_color_aliases_and_unscaled_failure_prefix |
| light strength descriptor 00A9FCA8, effects §19.33 | Commands::shake_light_curve(Light,0x0C) | ec153_through_ec160_curves_color_aliases_and_unscaled_failure_prefix |
| light color descriptor 00A9FC88, effects §19.33 | Commands::light_color | ec153_through_ec160_curves_color_aliases_and_unscaled_failure_prefix |
| zoom descriptor 00A9FB78, §19.18 | `Commands::particle_zoom_stretch(false)` | `particle::tests::ec72_through_ec78_raw_scalar_and_life` |
| stretch descriptor 00A9FAC8, §19.18 | `Commands::particle_zoom_stretch(true)` | same EC72–78 test |
| cParticleLifeCommand, §19.18 | `Commands::particle_life` | same EC72–78 test |
| cParticleRateCommand, §19.19 | `Commands::particle_rate(0)`, `particle_rate_selected`, `particle_rate_options` | `particle::tests::ec79_through_ec86_rate_prefix_modes_and_options` |
| cParticleInjectCommand, §19.19 | `Commands::particle_rate(2)` | same EC79–86 test |
| cParticleMaintainCommand, §19.19 | `Commands::particle_rate(4)` | same EC79–86 test |
| cParticleAlignmentCommand, §19.20 | `Commands::particle_align`, `particle::ALIGN` | `particle::tests::ec87_through_ec90_alignment_guard_and_aliases` |
| cParticleCollisionCommand, §19.22 | `Commands::particle_collision`, `particle::bounded` | `particle::tests::ec91_through_ec104_collision_terrain_and_walk` |
| cParticleTerrainRepelCommand, §19.22 | `Commands::particle_terrain` | same EC91–104 test |
| cParticleRandomWalkCommand, §19.23 | `Commands::particle_walk` | same EC91–104 test |
| cParticleColorCommand, §19.25 | `Commands::particle_appearance(Color)` | `particle::tests::ec105_through_ec119_appearance_precision_prefix_and_suffix` |
| alpha descriptor 00A9FB58, §19.25 | `Commands::particle_appearance(Alpha)` | same EC105–119 test |
| size descriptor 00A9FB48, §19.25 | `Commands::particle_appearance(Size)` | same EC105–119 test |
| aspect descriptor 00A9FB38, §19.25 | `Commands::particle_appearance(Aspect)` | same EC105–119 test |
| cParticleRotateCommand, §19.25 | `Commands::particle_appearance(Rotate)` | same EC105–119 test |
| cDecalEffectCommand, §19.35 | `nested::DECAL`, `nested::register` | `nested::tests::ec161_ec272_through_ec276_decal_and_group_dictionaries` |
| decal color descriptor 00A9FC28, §19.36 | `Commands::decal_appearance(Color)` | `resources::tests::ec162_through_ec175_decal_curves_and_life` |
| decal alpha descriptor 00A9FC18, §19.36 | `Commands::decal_appearance(Alpha)` | same EC162–175 test |
| cDecalSizeCommand, §19.36 | `Commands::decal_appearance(Size)` | same EC162–175 test |
| decal aspect descriptor 00A9FBF8, §19.36 | `Commands::decal_appearance(Aspect)` | same EC162–175 test |
| decal rotate descriptor 00A9FBE8, §19.36 | `Commands::decal_appearance(Rotate)` | same EC162–175 test |
| cDecalLifeCommand, §19.37 | `Commands::decal_life` | same EC162–175 test |
| cDecalTextureCommand, §19.37 | `Commands::decal_texture`, `decal_texture_options`, `texture_selected` | `resources::tests::ec176_through_ec183_decal_texture_flags_selection_and_failures` |
| particle draw helper, §19.39 | `Commands::particle_draw`, `resources::PARTICLE_DRAW` | `resources::tests::ec184_through_ec201_particle_resources` |
| cParticleTextureCommand, §19.39 | `Commands::particle_texture`, `texture_selected` | same EC184–201 test |
| cParticleModelCommand, §19.40 | `Commands::particle_model`, `particle_model_options`, `particle_model_selected` | same EC184–201 test |
| timedEffect descriptor 00A9FA18, §19.41 | `Commands::particle_timed`, `Description::timed_effects` | `resources::tests::ec202_through_ec208_timed_effect_insertion_and_unordered_times` |
| cParticleWarpCommand, §19.43 | `Commands::particle_warp`, `Description::wiggles` | `motion::tests::ec209_through_ec219_warp_order_atomic_append_and_uv_boundary`; `ec220_through_ec223_warp_alpha_prefix_and_nan` |
| cParticleSourceCommand, §19.45 | `Commands::particle_source` | `motion::tests::ec224_through_ec233_source_shapes_and_short_circuit`; `ec234_through_ec236_source_secondary_order_and_failure_prefix` |
| cParticleEmitCommand, §19.47 | `Commands::particle_emit`, `particle_rate_options` | `motion::tests::ec237_through_ec242_emit_rates_and_velocity`; `ec243_through_ec249_emit_direction_shapes_and_overrides` |
| cParticleForceCommand, §19.49 | `Commands::particle_force`, `force_list` | `motion::tests::ec250_through_ec258_force_acceleration_and_attraction_failure_prefix`; `ec259_through_ec264_force_automata_terrain_and_explosion` |
| cParticleForceCommand, §19.50 | `Commands::tractor`, `particle_force`, `Description::tractors` | `motion::tests::ec265_through_ec271_force_tractor_geometry_time_and_compounding`; definition EC57 inherited tail fixture |
| cEffectGroupIDCommand, §19.52 | `nested::EFFECT_GROUP`, `Commands::effect_id(true)` | nested EC275 test checks literal equality with `ROOT_COMMANDS[7]`; existing EC5–15 execution tests |
| cGroupEffectCommand, §19.52 | `nested::EFFECT`, `nested::register` | nested EC272–276 test |
| cGroupParticlesCommand, §§19.52–53 | `nested::EFFECT[0]`, `Commands::named_component(Particle)` | nested EC272–276; `components::tests::ec277_through_ec282_named_context_validation_shells_and_flags`, `named_common_options_keep_store_order_and_zero_shell_comparison` |
| cGroupDynamicParticleCommand, §§19.52–53 | `nested::EFFECT[1]`, `Commands::named_component(Dynamic)` | same EC277–282 and common-option tests |
| cGroupDecalCommand, §§19.52–53 | `nested::EFFECT[2]`, `Commands::named_component(Decal)` | same EC277–282 tests |
| cGroupShakeCommand, §§19.52,19.54 | `nested::EFFECT[3]`, `Commands::auxiliary_component(Shake)` | `ec283_through_ec287_auxiliary_order_alias_short_circuit_and_failure_prefix` |
| cGroupFlashCommand, §§19.52,19.54 | `nested::EFFECT[4]`, `Commands::auxiliary_component(Flash)` | same EC283–287 tests |
| cGroupTintCommand, §§19.52,19.54 | `nested::EFFECT[5]`, `Commands::auxiliary_component(Tint)` | same EC283–287 tests |
| cGroupSoundCommand, §§19.52,19.55 | `nested::EFFECT[6]`, `Commands::anonymous_component(Sound)`, `sound_payload` | `ec288_through_ec290_sound_brush_maps_and_stored_length`, `anonymous_failure_prefix_ranges_missing_names_and_nonfinite_lengths` |
| chainEffect descriptor 00A9FE3C, §§19.52,19.54 | `nested::EFFECT[7]`, `Commands::chain_component` | EC283–287 tests |
| cGroupBrushCommand, §§19.52,19.55 | `nested::EFFECT[8]`, `Commands::anonymous_component(Brush)`, `brush_payload` | EC288–290 and anonymous failure-prefix tests |
| cGroupScrubberCommand (four aliases), §§19.52,19.55 | `nested::EFFECT[9..13]`, `Commands::anonymous_component(Scrubber)`, `scrubber_payload` | `ec291_through_ec293_scrubber_aliases_crt_and_shape_overrides`; all four literal aliases also checked by registration tests |
| cGroupAttractorCommand, §§19.52,19.55 | `nested::EFFECT[13]`, `Commands::anonymous_component(Automata)` | `ec294_through_ec297_ec305_automata_camera_and_anonymous_insertion_prefix` |
| cGroupSequenceCommand, §19.52 | `nested::EFFECT[14]`, `Parser::sequence_effect` | nested registration test; effects-parser §6.2 execution tests above |
| cGroupCameraCommand, §§19.52,19.55 | `nested::EFFECT[15]`, `Commands::anonymous_component(Camera)` | EC294–297/305 and anonymous failure-prefix tests |
| cGroupGroupCommand, §§19.52–53 | `nested::EFFECT[16]`, `Commands::named_component(Visual)` | EC277–282 tests |
| select block descriptor 00A9FDC0, §§19.52,19.56 | `nested::EFFECT[17]`, `Commands::enter_select`, `exit_select` | `ec298_through_ec304_select_counts_tail_wrap_nesting_and_hooks` |
| particleSequence block descriptor 00A9FED4, §§19.52,19.56 | `nested::EFFECT[18]`, `Commands::enter_particle_sequence`, `exit_particle_sequence` | same EC298–304 tests |

The conformance sections map to the test groups above without interpreting them
as native golden runs: §19.3 EC1–6; §19.6 EC7–24; §19.8 EC25–34;
§19.10 EC35–42; §19.12 EC43–50; §19.15 EC51–64; §19.16 EC65–68;
§19.17 EC69–71; §19.21 EC72–90; §19.24 EC91–104; §19.26 EC105–119;
§19.28 EC120–130; §19.30 EC131–140; §19.34 EC141–160;
§19.38 EC161–183; §19.42 EC184–208; §19.44 EC209–223;
§19.46 EC224–236; §19.48 EC237–249; §19.51 EC250–271;
§19.52 EC272–276; §19.53 EC277–282; §19.54 EC283–287;
§19.55 EC288–297/305; §19.56 EC298–304. Shared input contract §19.1 uses the reader, error and
dispatch fixtures from effects-parser §§9–12 above. Typed timed-effect,
wiggle and tractor lists clone on inheritance and clear on description reset;
the generic integer lists are not used to encode their payloads.

Root conformance groups implement the static EC1–EC50 expectations with
individual primitive/service replies. They are portable tests, not game or
native-parser executions. Commands enter/exit methods return before the generic
caller pushes/pops a block; failed methods must never trigger a stack mutation.
The new script byte-token boundary coexists with the original immutable
primitive-input option ledgers; neither introduces lexing or catch/recovery.

## effects.md §17 conformance scope

§17 assigns runtime behavior to the manager, brush and particle owners, not the
script command classes. Existing coverage is recorded here so runtime vectors
are not mistaken for command conformance. Nested script construction is now
covered independently by §§19.53–56 above; runtime/data census assertions remain
outside the command owners.

| effects §17 item | Existing code | Existing test/vector |
|---|---|---|
| 1 brush stroke | `effects::BrushEffect` | `ef17_01_brush_stroke`; `effects::tests::t01_brush_stroke_count` |
| 2 dilation | `effects::BrushEffect`, manager tick | `ef17_02_dilation`; `effects::tests::t02_dilation` |
| 3 select draw | `effects::EffectsManager` select groups | `ef17_03_select_draw`; `effects::tests::t03_select_draw` |
| 4 one-shot crater | `effects::BrushEffect` | `ef17_04_one_shot_crater`; `effects::tests::t04_one_shot_crater` |
| 5 bounds | `effects::BrushEffect` | `ef17_05_out_of_bounds`; `effects::tests::t05_out_of_bounds` |
| 6 game action order | `effects::GameEffect` | `ef17_06_game_record_order`; `effects::tests::t06_game_record_order` |
| 7 cell movement | `effects::GameEffect` | `ef17_07_move_to_new_cell`; `effects::tests::t07_move_to_new_cell` |
| 8 pause | `effects::GameEffect` | `ef17_08_pause_record`; `effects::tests::t08_pause_record` |
| 9 priority | `effects::EffectsManager` | `ef17_09_priority_filter`; `effects::tests::t09_priority_filter` |
| 10 draw-only finish | `effects::EffectsManager::finish_hook`, `draw_pass` | `ef17_10_no_end_without_the_draw_pass`; `effects::tests::t10_no_end_without_the_draw_pass` |
| 11 finish rules | manager finish | `effects::tests::finish_test_rules` |
| 12 description/runtime flags | manager component creation | `effects::tests::finish_test_rules` |
| 13 shipped flag census | directory records | No complete census assertion in parser tests; runtime/data test, outside command scope |
| 14 shipped LOD census | directory records | `effects::tests::lod_ranges_and_zoom` checks behavior, not the complete shipped census; outside command scope |
| 15 volcano finish | manager nested finish | `effects::tests::volcano_finishes_in_the_draw_pass_or_the_hook` |
| 16 particle self-stop | `effects::particles` | `ef17_16_particle_self_stop`; `effects::particles::tests::loop_budget_stops_delayed_and_plays_out` |
| 17 dilation table | `effects::dilation` | `ef17_17_dilation_table`; `effects::tests::t17_dilation_table` |

The §17 vectors are in `engine/conformance/vectors`, run by
`engine/conformance/src/effects.rs`. No manager replacement or game request is
introduced by this command audit. All execution evidence here is portable core
testing; no native command match is claimed.
