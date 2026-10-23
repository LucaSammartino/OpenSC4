# Lua host core

`ScriptServer` implements the script-server owner through foreign interfaces; its
existing inventory is in [OWNER.md](OWNER.md). `collectors` implements the two
extension collector owners in lua-host.md §8.1 and provides their §8.2 registration
consumer. There is one immutable process-wide registry, initialized in simulation
then UI order. It has 47 entries: 35 simulation `game` callbacks, three globals,
then nine UI `game` callbacks. Nothing is persisted.

`Function` is a symbolic callback identity. `RegistrationVm` binds it to the foreign
callback and exposes primitive stack operations. The core neither executes Lua nor
implements game services. The registration consumer uses unprotected `SetTable`;
an adapter must preserve Lua's panic path for an absent/non-table `game`. A foreign
error propagates without pretending registration succeeded. Successful registration
always returns true and restores the incoming stack top. `_tf_` remains a real
global: `print` after global registration, `window_graph` after game registration.

## Owner closure

Test names below are in `engine/conformance/tests/lua_collectors.rs`; they run as
the `lua_collectors` integration target of `fc4-conformance`.

| Owner | Spec section | Code item | Conformance test |
|---|---|---|---|
| cLUAExtensionsCollector | lua-host §8.1 (38 entries; names/order cross-reference advisors §8.2–8.3) | `SimulationCollector::append_to`, `extensions` | `collector_order_flags_identity_and_shared_process_registry`, `both_collectors_append_without_clearing_existing_entries` |
| cLUAExtensionsCollectorUI | lua-host §8.1 (nine entries, shared vector, constructed second) | `UiCollector::append_to`, `extensions` | `collector_order_flags_identity_and_shared_process_registry`, `both_collectors_append_without_clearing_existing_entries` |
| cLUAExtensionsCollector | lua-host §8.2 (registration consumer of collected entries) | `register_api`, `RegistrationVm` | `t9_register_api_globals_and_all_44_game_names`, `missing_game_reaches_unprotected_set_table_and_does_not_fake_success`, `unchanged_target_top_skips_assignment_and_still_restores_top` |
| cLUAExtensionsCollectorUI | lua-host §8.2 (ordered game registration, last scratch function) | `register_api`, `RegistrationVm` | `t9_register_api_globals_and_all_44_game_names` |
| cLUAExtensionsCollector | lua-host §8.3 (collected global identities) | `SimulationCollector::append_to`, `Function` | `collector_order_flags_identity_and_shared_process_registry`, `t9_register_api_globals_and_all_44_game_names` |
| cLUAExtensionsCollector | lua-host §16 T9 (three globals, true return, `_tf_ = print`) | `register_api` | `t9_register_api_globals_and_all_44_game_names` |
| cLUAExtensionsCollectorUI | lua-host §16 T9 (44 game names, true return, `_tf_ = game.window_graph`) | `register_api` | `t9_register_api_globals_and_all_44_game_names` |

No collector behaviour is open or Unsupported. Callback bodies belong to their
host/advisor/game-service owners: collecting `print`, `_dofile_from_resource`, and
`_combine_guids` registers their foreign identities rather than running them.
Thus §16 T8/T12 concern other owners; T9 is the collector registration test. Tags,
registry mirror, resource loading, detokenization, and VM operations are also
separate owners. No plugin/C ABI wiring or real-game execution is involved here.

### cGZLua5 owners

`vm::LuaHost`, `vm::Thread`, `vm::Session` and `vm::generator::CommandGenerator`
extend this module. `LuaApi` supplies Lua 5.0 C API, storage, precision and logging
primitives. It never supplies a host-level Tag/GetN/CallProtected/Open result.
`Session::execute(Op)` is the complete thin push/pull/table/environment/load/call
surface; composite host wrappers have named methods. Foreign Lua errors must
unwind to the current protected call or follow Lua's panic/exit path. Core returns
an explicit invalid-precondition error for the native null userdata dereference.

`Session::get_n` is the cSCLua-specific slot-64 convenience algorithm hosted on
the same stack session; it is not an additional cIGZLua5Thread vtable slot.
Similarly, the ignored reference argument is the cSCLua forwarding convention;
the thread reference operation itself takes only the registry's top value.

The adapter supplies standard Lua 5.0 `etc/compat.lua` as immutable library data;
it executes in the foreign interpreter. Core registers and implements the seven
tag/global callbacks, owns the process-wide atomic tag counter and overrides
coroutine create/resume with callbacks routed to `LuaHost::create_from` and
`resume_from`. Session-only callback dispatch rejects these owner callbacks;
the library adapter performs standard Lua coroutine argument/result handling.
The standard Lua coroutine library supplies the remaining coroutine functions.
Precision primitives let an adapter use the actual x87 control word; the tests
record 53-bit execution and restoration from 24-bit contexts, including errors.
`Unref` deliberately bypasses that guard. Rust 1.75 remains sufficient.

LH4 and LH5 are implemented. `lifecycle` owns records, flags, monotonic IDs,
parent/child destruction, stack movement, result counts and reference decisions.
Thread views share synchronized state, including invalidation and the allocation
reference count; cloning a Rust view borrows and does not AddRef. `ThreadApi`
supplies factory/storage identities and overridable thread hooks, with concrete
core defaults. Interpreter primitives remain `LuaApi` operations, including
resume, yield and activation-frame inspection. Explicit `release`/`destroy`
methods accept the foreign API because Rust Drop cannot perform foreign calls.
Clients must Close an open host to release its self-held references.

The generator owns 500 initial default records, grows by 20, reuses free indices
from the front, captures unowned identities, converts command parameters and
preserves output enumeration order. Its foreign boundary supplies command and
variant primitives, never setup/registration/dispatch decisions. Successful Init
does not enumerate commands, and Shutdown does not remove globals on its own.
Only the spec's existing unsettled interpreter identity and default reporter
output/text remain open (§15 Q1/Q2/Q5/Q8); the reporter uses the engine-log choice.

The table has one row per major spec section for each owner; relevant subsections
and open portions appear in their row. Test names refer to
`engine/conformance/tests/lua_vm.rs` (including `support/lua_vm_lifecycle.rs`).
Sections assigned only to other owners are
explicitly excluded, so their tests are not claimed as VM/thread coverage.

| Owner | Spec section | Code item | Test / open boundary |
|---|---|---|---|
| cGZLua5 | §1 identity/backpointer | `Thread::supports`, `LuaHost::open`, `query_interface` | `identities_and_explicit_open_lifecycle_boundaries`, `lh4_a_b_c_e_main_child_roots_borrowing_and_monotonic_ids`, `shared_unknown_counts_owner_changes_and_destructors` |
| cGZLua5Thread | §1 identity/backpointer | `Thread::supports`, `query_interface`, `ThreadKind` | `identities_and_explicit_open_lifecycle_boundaries`, `shared_unknown_counts_owner_changes_and_destructors` |
| cGZLua5CommandGenerator | §1 identity | `CommandGenerator::supports`, `as_command_generator`, `query_interface` | `identities_and_explicit_open_lifecycle_boundaries`, `shared_unknown_counts_owner_changes_and_destructors` |
| cGZLua5 | §2 defaults/persistence | `LuaHost::new`, `error_reporter`; no persisted fields | `base_open_flags_repeat_failures_close_and_default_reporter`, `lh4_a_b_c_e_main_child_roots_borrowing_and_monotonic_ids` |
| cGZLua5Thread | §2 state | `Thread::new`, synchronized context/owner/user context | `identities_and_explicit_open_lifecycle_boundaries`, `lh4_k_l_hooks_parent_restrictions_recursive_destroy_and_pause` |
| cGZLua5CommandGenerator | §2 | `CommandGenerator::new` | `lh5_a_b_c_d_reinitialization_null_sets_and_global_identity` |
| cGZLua5 | §3.1–3.2 base VM lifecycle | `LuaHost::open`, `close` | `base_open_flags_repeat_failures_close_and_default_reporter`, `lh4_d_f_failed_factory_and_hooks_do_not_rollback_or_advance_id`; cSCLua subscription/parameter sets belong to cSCLua |
| cGZLua5Thread | §3.3–3.4 compatibility | `Session::open_compatibility`, `callback_inner`, `metatable` | `t10_process_tags_and_absolute_index_quirk`, `tag_methods_previous_value_unmapped_error_and_string_only_copy`, `compatibility_failure_is_ignored_and_alert_only_reports_strings` |
| cGZLua5CommandGenerator | §3 | VM operations consumed through its borrowed main thread | `generator_validation_retention_and_shutdown_order`, `lh5_a_b_c_d_reinitialization_null_sets_and_global_identity` |
| cGZLua5 | §4.3/4.5/4.6 reporter/precision/GC | `set_error_reporter`, `guarded`, GC methods | `base_open_flags_repeat_failures_close_and_default_reporter`, `runtime_dobuffer_error_gc_and_load_without_execution`; reporter output/text open: spec §15 Q5/Q8 |
| cGZLua5Thread | §4.1–4.6 calls/errors/precision/refs | `protected_inner`, `do_inner`, `report`, `execute`, `reference`, `get_ref`, `unref` | `t13_traceback_handler_report_and_pop_preserve_prefix`, `protected_explicit_handler_disabled_reporter_and_retained_error`, `do_functions_no_traceback_alert_status_one_and_stderr_fallback`, `t18_foreign_double_result_and_precision_restore_on_success_and_failure`, `boxed_pointer_tags_and_registry_reference_lifetime` |
| cGZLua5CommandGenerator | §4 | No additional generator-specific rules assigned; callback returns command status | `lh5_h_i_j_parameter_types_order_reserved_key_status_and_validation` |
| cGZLua5 | §5 main-thread forwards/GC | `LuaHost::main`, GC methods; shared `Session` surface | `runtime_dobuffer_error_gc_and_load_without_execution`; cSCLua-specific slots excluded |
| cGZLua5Thread | §5/5.1 shared wrappers | `Session::execute`, named tag/conversion/global/registry wrappers | `registry_conversion_and_ignored_arguments_and_invalid_paths`, `t10_process_tags_and_absolute_index_quirk`, `boxed_pointer_tags_and_registry_reference_lifetime`; cSCLua-specific GetN has an additional T11 helper test |
| cGZLua5CommandGenerator | §5 | No generator behaviour assigned | Not applicable |
| cGZLua5 | §6.1 | `LuaHost` main/GC/reporter/create/destroy/get/resume methods | `base_open_flags_repeat_failures_close_and_default_reporter`, all `lh4_*` tests |
| cGZLua5Thread | §6.2 / §6.2.1 construction/hooks/ownership | `Thread::new`, `on_create`, `on_destroy`, `query_interface`, `add_ref`, `release`, `LuaHost::open/close/destroy` | `identities_and_explicit_open_lifecycle_boundaries`, `lh4_d_f_failed_factory_and_hooks_do_not_rollback_or_advance_id`, `shared_unknown_counts_owner_changes_and_destructors` |
| cGZLua5Thread | §6.2.2 children/IDs/destruction | `LuaHost::create_from`, `destroy_from`, `get_thread`, `Record` | `lh4_a_b_c_e_main_child_roots_borrowing_and_monotonic_ids`, `lh4_d_f_failed_factory_and_hooks_do_not_rollback_or_advance_id`, `lh4_k_l_hooks_parent_restrictions_recursive_destroy_and_pause`, `thread_hook_failures_parent_validation_and_destroy_order` |
| cGZLua5Thread | §6.2.3 status/yield/resume | `Thread::is_valid`, `pause`, `LuaHost::status`, `resume_from`, `resume_thread` | `lh4_g_h_i_j_resume_results_yield_completion_rejection_and_reporting`, `lh4_k_l_hooks_parent_restrictions_recursive_destroy_and_pause` |
| cGZLua5CommandGenerator | §6.3.2.1 construction/init | `CommandGenerator::new`, `generic_init`, `init` | `lh5_a_b_c_d_reinitialization_null_sets_and_global_identity`, `generator_validation_retention_and_shutdown_order` |
| cGZLua5CommandGenerator | §6.3.2.2 notifications/callbacks | `command_registered`, `command_unregistered`, `callback` | `lh5_e_f_g_registration_collisions_saved_closures_and_index_reuse`, `lh5_h_i_j_parameter_types_order_reserved_key_status_and_validation` |
| cGZLua5CommandGenerator | §6.3.2.3 cleanup/failure boundaries | `shutdown`, `destroy`, `release` | `lh5_k_l_shutdown_failure_repeat_and_destructor_do_not_remove_bindings`, `shared_unknown_counts_owner_changes_and_destructors` |
| cGZLua5 | §7 resources | Other owner: cSCLua/resource helpers | Not assigned; T1–T8/T14 not claimed |
| cGZLua5Thread | §7 resources | Foreign Load primitives consumed by resource owner | No resource-owner closure claimed |
| cGZLua5CommandGenerator | §7 resources | No generator behaviour assigned | Not applicable |
| cGZLua5 | §8 extension registration | Other owner: collectors/cSCLua callbacks | Existing collector table above; T9/T12 not claimed here |
| cGZLua5Thread | §8 extension registration | Exposes primitive stack operations to foreign C callbacks | Callback bodies belong to their named owners |
| cGZLua5CommandGenerator | §8 extension registration | No generator behaviour assigned | Not applicable |
| cGZLua5 | §9 registry mirror | Other owner: cSCLua | T16 not claimed |
| cGZLua5Thread | §9 registry mirror | `Session::get_upvalue_index`, userdata/table operations | `registry_conversion_and_ignored_arguments_and_invalid_paths`; mirror closure belongs to cSCLua |
| cGZLua5CommandGenerator | §9 registry mirror | No generator behaviour assigned | Not applicable |
| cGZLua5 | §10 detokenizer | Other owner: cSCLua | T15 not claimed |
| cGZLua5Thread | §10 detokenizer | No additional thread behaviour assigned | Not applicable |
| cGZLua5CommandGenerator | §10 detokenizer | No generator behaviour assigned | Not applicable |
| cGZLua5 | §11 call sites | No new owner algorithms assigned | Main view/foreign primitives as above |
| cGZLua5Thread | §11 call sites | No new owner algorithms assigned | Same shared stack surface |
| cGZLua5CommandGenerator | §11 call sites | No additional generator contract | `generator_validation_retention_and_shutdown_order` |
| cGZLua5 | §12 dependencies | `LuaApi`, `Library` | `base_open_flags_repeat_failures_close_and_default_reporter` |
| cGZLua5Thread | §12 dependencies | `LuaApi` interpreter boundary | `t18_foreign_double_result_and_precision_restore_on_success_and_failure` |
| cGZLua5CommandGenerator | §12 dependencies | `GeneratorApi` command-service boundary | `generator_validation_retention_and_shutdown_order` |
| cGZLua5 | §13 replaceability | Core identities and state backpointer primitive | `base_open_flags_repeat_failures_close_and_default_reporter`; native adapter intentionally absent |
| cGZLua5Thread | §13 replaceability | Process-wide tag counter, shared wrappers, core registry | `t10_process_tags_and_absolute_index_quirk`, all `lh4_*` tests |
| cGZLua5CommandGenerator | §13 replaceability | `GeneratorApi` primitives, core binding/dispatch decisions | all `lh5_*` tests |
| cGZLua5 | §14 quirks | Shared Session implementation; cSCLua-only quirks excluded | See thread row; `lh4_d_f_failed_factory_and_hooks_do_not_rollback_or_advance_id` |
| cGZLua5Thread | §14 Q1/Q3/Q4/Q6/Q14 | absolute-index `new_tag`, boxed pointer conversion, ignored Error argument, standalone QI, unguarded Unref | `t10_process_tags_and_absolute_index_quirk`, `registry_conversion_and_ignored_arguments_and_invalid_paths`, `identities_and_explicit_open_lifecycle_boundaries`, `t18_foreign_double_result_and_precision_restore_on_success_and_failure`; Q2/GetN and ignored Ref argument are cSCLua convenience coverage |
| cGZLua5CommandGenerator | §14 quirks | No generator-specific quirk assigned | Not applicable |
| cGZLua5 | §15 open questions | Engine-log choice for default reporter; output/text unsettled | open: spec §15 Q5/Q8 |
| cGZLua5Thread | §15 open questions | Panic belongs to foreign Lua (exit 1 regardless of panic identity); Lua version differences not relied on | `registry_conversion_and_ignored_arguments_and_invalid_paths`; open: spec §15 Q1/Q2 (no speculative panic implementation) |
| cGZLua5CommandGenerator | §15 open questions | No generator behavior marked open | all `lh5_*` tests |
| cGZLua5 | §16 T17/T18 | Base lifecycle and precision only | `base_open_flags_repeat_failures_close_and_default_reporter`, `t18_foreign_double_result_and_precision_restore_on_success_and_failure`; cSCLua T17 registry-subscription semantics excluded |
| cGZLua5Thread | §16 T10/T13/T18 | Actual core callbacks/wrappers against a stack/table fixture | Tests prefixed `t10`, `t13`, `t18`; T11 additionally covers the cSCLua convenience helper; arithmetic/execution outcomes supplied by foreign fixture, not an interpreter conformance claim |
| cGZLua5 | §16.1 LH4 A–L | Core registry and lifecycle | all `lh4_*` tests; additional hook/refcount discriminators named above |
| cGZLua5Thread | §16.1 LH4 A–L | Shared thread state and concrete hooks | all `lh4_*` tests; additional hook/refcount discriminators named above |
| cGZLua5CommandGenerator | §16.1 LH5 A–L | Full setup/binding/callback/cleanup lifecycle | all `lh5_*` tests |

No game request is needed for this portable core-only change: there is no plugin
or C ABI wiring and no claim of a real-game shadow comparison.
