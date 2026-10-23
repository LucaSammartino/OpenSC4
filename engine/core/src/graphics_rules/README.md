# Graphics rules

Portable implementation of `docs/specs/graphics-rules.md`. Reuses the existing
`graphics_rules` module and the production `RenderProperties` registry. No plugin
or C ABI binding is added. The manager owns parsing, evaluation, recommendation
updates, fallback construction and logging decisions. `InformationProvider`
supplies explicit OS/driver reports and the rendering probe result; `ScriptFiles`
supplies file contents; `Lowercase` supplies foreign byte folding; `LogFile`
supplies file acquisition, open state and raw writes. None supplies rule results.

The native COM factories, secondary-interface pointer adjustments and refcount
thunks belong to a future binding. Core identities are `CLASS_ID`/`INTERFACE_ID`;
owned lists and retained properties use Rust ownership and `Arc<Mutex<_>>`.

## Owner closure

All settled manager behavior is implemented below. Complete-owner compatibility
remains open where the spec explicitly leaves shared dependencies unsettled:
GR4 (shared grammar/command bodies), GR6 (exceptional CRT formatting and unsafe
failure outcomes). GR7 asks which active x87 profiles are required; GR8 records
the contradictory label width in §6.1. These are reviewable portable-core changes,
not a claim of native acquisition, game shadow, default or replacement parity.

Test names below are in `tests.rs` and `closure_tests.rs` in this directory.

| Owner | Spec section | Code item | Covering test / open boundary |
|---|---|---|---|
| cSC4ConfigRuleManager | §0 summary | `init`, `set_options_from_configuration`, `apply_option_group_setting` | `v5_dense_defaults_reopen_and_recommendations_are_not_application`, `log_filename_preferences_property_dump_and_failed_open_evaluation` |
| cSC4ConfigRuleManager | §1 identity, ownership, typed sets | `CLASS_ID`, `INTERFACE_ID`, `new`, `init`, `PropertySet::apply` | `ownership_defaults_reinit_same_and_replacement_are_counted`, `typed_lookup_precedence_duplicates_and_setter_failures`; native thunks are binding scope |
| cSC4ConfigRuleManager | §2 variables | `Environment::default`, `Environment::gather` | `v7_record_flags_units_unsigned_conversions_and_bogo_defaults` |
| cSC4ConfigRuleManager | §2.1 provider boundary | `Measurements`, `InformationProvider`, `forced_fsaa_from_samples` | `v7_readback_provider_helper_ignores_alpha_and_needs_two_samples`, `v7_record_flags_units_unsigned_conversions_and_bogo_defaults` |
| cSC4ConfigRuleManager | §2.2 regather/unavailable services | `gather_information`, `Environment::gather` | `v13_lf_exhaustion_retains_cr_and_identity_and_permissive_ids`, `v10_explicit_logging_regathers_without_evaluation_and_failed_open_still_gathers`; mandatory properties/provider are preconditions |
| cSC4ConfigRuleManager | §3 reading rules | `parse_config_file`, `InitContext`, `card_ids`, `RuleParser::parse_files` | `v9_vendor_latch_fixed_offsets_and_unmatched_vendor_reset`, `v14_optional_error_keeps_callbacks_and_required_error_visits_later_roots`, `v10_errors_never_publish_partial_lists_and_init_installs_fallback` |
| cSC4ConfigRuleManager | §4 language dependency | `RuleParser`, `PropertySet` | `lexical_literal_subset_comments_symbols_includes_and_optional_user_order`; open: GR4 complete shared grammar |
| cSC4ConfigRuleManager | §4.1 option groups | `OptionGroup`, `grow`, `apply_option_group_setting` | `v5_dense_defaults_reopen_and_recommendations_are_not_application`; open: GR6 unsafe allocation/index outcomes |
| cSC4ConfigRuleManager | §4.2 rule records/matching | `Rule`, `Rule::matches`, `threshold_from_real`, `wildcard_match` | `v1_thresholds_and_typed_write_order`, `v2_flags_aggregate_empty_modes`, `v3_wildcards_and_negative_polarity`, `condition_storage_and_short_circuit_order`, `gr5_nan_follows_main_spec_all_pass_any_no_match` |
| cSC4ConfigRuleManager | §4.3 typed values | `add_property`, `signed`, `real` | `typed_lookup_precedence_duplicates_and_setter_failures`, `v11_arithmetic_precedence_conversions_and_dispatch`; open: GR4 boolean aliases/exceptional conversions, GR7 active x87 profile |
| cSC4ConfigRuleManager | §4.4 card identity | `begin_vendor`, card dispatch, `parse_config_file` | `v9_vendor_latch_fixed_offsets_and_unmatched_vendor_reset` |
| cSC4ConfigRuleManager | §4.5 shared parser facts | `expression::evaluate`, `substitute`, `dispatch`, `unsigned`, `signed` | `v11_arithmetic_precedence_conversions_and_dispatch`, `permissive_ids_and_explicit_unsafe_index_embedding_boundary`; open: GR4 full lexical/dispatch/typed-symbol/function contracts, GR6 unsafe outcomes, GR7 active x87 profile |
| cSC4ConfigRuleManager | §5 evaluation | `apply_rules` | `first_match_partial_and_multilevel_depth_stack`, `v4_nested_current_rule_is_restored_after_child_end`, `v6_force_software_clear_is_persistent_until_gather`, `recommendation_duplicates_last_win_and_option_apply_does_not_clear_flags` |
| cSC4ConfigRuleManager | §6 applying/logging | option getter/setter, `set_options_from_configuration`, `log_configuration_to_file`, `logging::configuration`, `logging::options` | `log_filename_preferences_property_dump_and_failed_open_evaluation`, `v10_explicit_logging_regathers_without_evaluation_and_failed_open_still_gathers` |
| cSC4ConfigRuleManager | §6.1 byte/write boundary | `LogFile`, `logging::line`, byte-name `Properties::dump`, `general` | `v15_empty_options_raw_names_nuls_two_writes_and_padding`, `v15_write_and_close_failures_are_ignored_and_open_is_checked_per_line`; open: GR6 nonfinite `%g`, GR8 21-byte label versus 20-byte field |
| cSC4ConfigRuleManager | §7 fallback | `set_fallback_rules` | `v8_exact_fallback_rules_and_boundary_stage_two`, `v15_write_and_close_failures_are_ignored_and_open_is_checked_per_line` (unknown lookup sentinel is still dispatched) |
| cSC4ConfigRuleManager | §8 users | public manager operations | `v5_dense_defaults_reopen_and_recommendations_are_not_application`; caller wiring is outside this portable module |
| cSC4ConfigRuleManager | §9 quirks | `apply_rules`, `card_ids`, `begin_rule`, `Rule::matches` | `first_match_partial_and_multilevel_depth_stack`, `unnamed_rules_use_last_emitted_name_and_switch_alias`, `v9_vendor_latch_fixed_offsets_and_unmatched_vendor_reset`, `v2_flags_aggregate_empty_modes` |
| cSC4ConfigRuleManager | §10 open questions | explicit supported-profile boundaries below | open: GR4, GR6, GR7, GR8 |
| cSC4ConfigRuleManager | §11 conformance | manager/parser/evaluator/logging | eight numbered cases mapped below |
| cSC4ConfigRuleManager | §12 manager inventory | all public manager operations, `PropertySet`, `Environment`, `Rule` | tests in preceding rows; no omitted settled manager operation |
| cSC4ConfigRuleManager | §12.1 concrete dependencies | expression/parser bodies; byte logger; fallback unknown-index storage | V11–V15 tests; open: GR4 complete shared bodies, GR6 exceptional formatting/failure outcomes, GR7 active controls |
| cSC4RuleParser | §§3, 4, 12 | `new`, `parse_files`, `source`, `finish`, `parse_config_file` transfer | `v14_optional_error_keeps_callbacks_and_required_error_visits_later_roots`, `parse_success_replaces_both_lists_and_uint32_recommendations_keep_bits`; open: GR4 full shared grammar |
| cRuleCommand | §4.2 / §12 | `begin_rule`, `current_rule`, block dispatch | `v4_nested_current_rule_is_restored_after_child_end`, `conditions_after_child_target_parent_and_recommendations_persist_when_nothing_matches`, `unnamed_rules_use_last_emitted_name_and_switch_alias` |
| cOptionGroupCommand | §4.1 / §12 | group/option/property dispatch | `v5_dense_defaults_reopen_and_recommendations_are_not_application`; open: GR6 unsafe outcomes |
| cVendorCommand | §4.4 / §12 | `begin_vendor`, card dispatch | `v9_vendor_latch_fixed_offsets_and_unmatched_vendor_reset` |
| AddOptionToSet | §§1, 4.3, 12 | `add_property`, `PropertySet::apply` | `typed_lookup_precedence_duplicates_and_setter_failures`; open: GR4 full conversion contract, GR7 active controls |

## Spec §11 conformance cases

| Case | Test |
|---|---|
| 1 first match, memory 1024/256 | `section11_first_match_partial_any_and_skipped_children` |
| 2 both partial siblings | `section11_first_match_partial_any_and_skipped_children` |
| 3 any/MMX with memory 512 | `section11_first_match_partial_any_and_skipped_children` |
| 4 failed-parent child skipped; action-only any never matches | `section11_first_match_partial_any_and_skipped_children` |
| 5 group 2 recommends 3; group 99 returns 0 | `v5_dense_defaults_reopen_and_recommendations_are_not_application` |
| 6 unknown property is parse error | `v10_errors_never_publish_partial_lists_and_init_installs_fallback` |
| 7 forced software clears hardware flag | `v6_force_software_clear_is_persistent_until_gather` |
| 8 ALPHA log path | `log_filename_preferences_property_dump_and_failed_open_evaluation` |

The independent fixtures in `docs/specs/graphics-rules-vectors.md` V1–V15 are
covered by the corresponding named tests (V12 extends the GR5 NaN test). These
are executed core conformance tests, not an extension of the unrelated simulator
JSON runner and not evidence of a native run.

## Supported-profile limits

- GR4: published arithmetic precedence, interpolation, eval and file-registration
  behavior are owned here. Full lexical tables, escapes, switch/arity/placement,
  boundary parenthesization, typed symbols, function/domain tables, generic
  command bodies, EOF and sinclude missing/cycle policies remain unsettled.
  Known unimplemented shared command names and numeric functions report
  `Unsupported GR4`; unknown commands retain the specified parse error. Existing
  literal token/EOF/include policies remain explicitly bounded to that subset.
- GR6: optional open failure is `ScriptFiles::read` returning `None`; other
  errors are recorded for an optional root without undoing callbacks. Failed
  ReadLine is EOF at this acquisition boundary. Unsafe native dense-index or
  allocation paths are safely rejected, without claiming native error parity.
  Mandatory provider/services must be supplied; there is no invented recovery.
- GR6: `general` returns `ManagerError::Unsupported` for nonfinite values.
  Logging closes resources and rule application still runs when this occurs.
  Supported finite numbers use six significant digits. Raw property names need
  no UTF-8 conversion; embedded NUL terminates `%s`, and `%` is ordinary data.
- GR7: signed-property conversion implements the published nearest/even bias
  profile, including behavior outside the small exact integer range. Other
  active x87 profiles are not claimed. Unsigned/threshold conversion uses the
  separately specified signed64 truncation/low32 operation.
- GR8: the literal 20-byte preference field truncates the 21-byte
  `VariableSpeedAutomata` label; other rows follow the published layout exactly.

Game validation is not applicable to this unwired core module. No game requests,
plugin changes, C ABI changes or coverage-map claims are made.
