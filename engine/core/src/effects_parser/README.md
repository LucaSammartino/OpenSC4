# Effects description parser

This module extends the existing `effects` component representation. It models
owner decisions using foreign primitive replies, not a text lexer or native
command interpreter. Renderer, sound, resource, conversion and generic command
library services remain foreign. No plugin or C ABI integration is provided.

The command-family closure is **open: EF15**. `effects.md` contains no command
class inventory or execution contracts. `effects-parser.md` §6.1 establishes
descriptor identities and registration associations, expressly excluding handler
algorithms. §6.2 specifies one handler, `sequenceEffect`. Exact `c*Command` class
names cannot be inferred from literals: the rows below identify each published
owner by descriptor and literal pending the class inventory requested in EF15.
These rows do not certify whole-class coverage.

## Owner closure

`root_registration_order_literals_descriptors_hooks_and_construction` (abbreviated
**root test** below) independently checks all 21 literal/descriptor/hook tuples,
their order, allocation size and block construction. All test items below are in
`tests.rs` or its included `extra_tests.rs` unless explicitly qualified.

| Command owner (descriptor; literal) | Spec section | Code item and test | Execution closure |
|---|---|---|---|
| `00A9F85C`; `renderprop` | effects-parser §6.1 | `ROOT_COMMANDS[0]`, `register_commands`; root test | open: EF15 |
| `00A9F86C`; `textureID` | effects-parser §6.1 | `ROOT_COMMANDS[1]`, `register_commands`; root test | open: EF15 |
| `00A9F87C`; `modelID` | effects-parser §6.1 | `ROOT_COMMANDS[2]`, `register_commands`; root test | open: EF15 |
| `00A9F88C`; `soundID` | effects-parser §6.1 | `ROOT_COMMANDS[3]`, `register_commands`; root test | open: EF15 |
| `00A9F89C`; `brushID` | effects-parser §6.1 | `ROOT_COMMANDS[4]`, `register_commands`; root test | open: EF15 |
| `00A9F8AC`; `effectID` | effects-parser §6.1 | `ROOT_COMMANDS[5]`, `register_commands`; root test | open: EF15 |
| `00A9F8BC`; `effectGroup` | effects-parser §6.1 | `ROOT_COMMANDS[6]`, `register_commands`; root test | open: EF15 |
| `00A9FA08`; `instance` | effects-parser §6.1 | `ROOT_COMMANDS[7]`, `register_commands`; root test | open: EF15 |
| `00A9F8D8`; `testEffect` | effects-parser §6.1 | `ROOT_COMMANDS[8]`, `register_commands`; root test | open: EF15 |
| `00A9F8E8`; `messageTrigger` | effects-parser §6.1 | `ROOT_COMMANDS[9]`, `register_commands`; root test | open: EF15 |
| `00A9F8F8`; `camera` | effects-parser §6.1 | `ROOT_COMMANDS[10]`, `register_commands`; root test | open: EF15 |
| `00A9F908`; `setPriority` | effects-parser §6.1 | `ROOT_COMMANDS[11]`, `register_commands`; root test | open: EF15 |
| `00A9F918`; `effectsResource` | effects-parser §6.1 | `ROOT_COMMANDS[12]`, `register_commands`; root test | open: EF15 |
| `00A9F934`; `loadResource` | effects-parser §6.1 | `ROOT_COMMANDS[13]`, `register_commands`; root test | open: EF15 |
| `00A9F944`; `effect` | effects-parser §6.1 | `ROOT_COMMANDS[14]`, `register_commands`; root test | open: EF15 |
| `00A9F960`; `particles` | effects-parser §6.1 | `ROOT_COMMANDS[15]`, `register_commands`; root test | open: EF15 |
| `00A9F97C`; `dynamicParticle` | effects-parser §6.1 | `ROOT_COMMANDS[16]`, `register_commands`; root test | open: EF15 |
| `00A9F998`; `decal` | effects-parser §6.1 | `ROOT_COMMANDS[17]`, `register_commands`; root test | open: EF15 |
| `00A9F9B4`; `shake` | effects-parser §6.1 | `ROOT_COMMANDS[18]`, `register_commands`; root test | open: EF15 |
| `00A9F9D0`; `light` | effects-parser §6.1 | `ROOT_COMMANDS[19]`, `register_commands`; root test | open: EF15 |
| `00A9F9EC`; `sequence` | effects-parser §6.1 | `ROOT_COMMANDS[20]`, `register_commands`; root test | open: EF15 |
| `00A9FDFC`; `sequenceEffect` | effects-parser §6.2 registration | `SEQUENCE_EFFECT`, `register_sequence_effect`; `nested_registration_and_allocation_boundary_are_separate` | published association covered |
| `00A9FDFC`; `sequenceEffect` | effects-parser §6.2 execution | `Parser::sequence_effect`; `r1_sequence_named_repeat_append`, `r2_options_override_initial_clear`, `unsupported_command_and_validation_keep_precise_prefix`, `sequence_command_reads_selection_before_null_manager_append_short_circuit`, `sequence_command_uses_only_first_selection_and_preserves_name_case` | normal primitive-input domain covered; error transport open: EFP3 |
| Other nested command owners (inventory unpublished) | effects-parser §6.2 exclusion; effects §§0–18 | open: EF15 | no inferred class names or handlers |

### Shared parser operations used by commands

| Owner / spec section | Code item | Tests / remaining open domain |
|---|---|---|
| Parser option selection; effects-parser §2 | `parse_options`, `OptionPrimitives` | `p1_absent_retains_asymmetric_fields`, `p7_duplicate_alternatives_are_distinct_foreign_fixtures`; grammar/consumption open: EFP1; rotations open: EFP2 |
| Parser nonrotation options; effects-parser §§3–4 | `parse_options`, `scalar`, `probability` | `p1_*` through `p8_*`, `scale_compares_original_after_rounded_store`, `forbidden_lod_and_probability_stop_at_error_boundary`; error/numeric qualification open: EFP3, EFP7 |
| Parser named append; effects-parser §5.1 | `Parser::add_named`, `Parser::finish` | `r1_sequence_named_repeat_append`, `r2_options_override_initial_clear`, `r5_null_manager_short_circuits_both_paths`, `r6_capacity_branch_is_after_mutation`, `named_rereads_shared_first_index_after_validation`; foreign identity open: EFP4 |
| Parser anonymous append; effects-parser §5.2 | `Parser::add_anonymous` | `r3_anonymous_sentinel_is_success_and_full_type_retained`, `r4_anonymous_raw_index_has_no_local_counter`, `r5_null_manager_short_circuits_both_paths`; foreign identity open: EFP4 |
| Shared record constructor; effects-parser §5.3 | `Record::shared_constructor_projection` | `shared_constructor_projection_is_not_direct_option_initialization` |
| Parser manager acquisition; effects-parser §5.4 | `Parser::set_manager` | `m1_distinct_manager_reference_order` through `m4_nonnull_manager_null_qi_clears_k`; reentry excluded by spec |
| Parser interface identity; effects-parser §5.5 | `query_interface`, `QueryIdentities` | `q1_logical_adjusted_identity_and_nullout`, `query_fallback_is_raw_and_missing_adjustment_unsupported`; destruction open: EFP5 |
| Command library; effects-parser §6.1 | `register_one`, `register_commands` | root test, `nested_registration_and_allocation_boundary_are_separate`; foreign dispatch/ownership open: EFP6 |
| Parser trace ledgers; effects-parser §7 | operations above | `r1_*`–`r6_*`, `m1_*`–`m4_*`, `q1_*`; stipulated foreign fixtures, not native golden executions |
| Coverage boundary; effects-parser §8 | module-level documentation, typed `Unsupported` | error-prefix tests retain earlier writes; whole parser remains partial as specified |

## effects.md §17 conformance scope

§17 assigns runtime behavior to the manager, brush and particle owners, not the
script command classes. Existing coverage is recorded here so runtime vectors
are not mistaken for command conformance. Nothing in this table establishes the
unpublished script that produces a runtime description (open: EF15).

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
