# GZ framework core

Source authority: `docs/specs/gz-framework-core.md`, with the command-system and
buffer bridge sections it cites. No plugin or C ABI wiring. `Token` is a foreign
identity; `References` records acquired references explicitly. OS path operations,
loading, application hooks, stream IO and unresolved ID/numeric policies stay
behind host traits. Core getters use ordinary borrows or locks, never RefCell.
The existing command parameter/message objects retain their established sharing.

## Owner closure

Every section is represented below. An open entry is a real specification boundary,
not a successful stub. `Unsupported("GZFn")` identifies the relevant question.
Conformance names are the tests in `tests.rs`; grouped names cover each listed
vector individually with assertions. This is closure of the **settled** contract;
the explicitly incomplete native inventory cannot claim binary replacement safety.

| Owner | Spec section | Code item | Test / open boundary |
|---|---|---|---|
| all nine | §1 | host traits and reused `commands`, `gz_graphics` consumer interfaces | Core-only compilation; no ABI |
| cGZFramework | §2.1 | `Framework::{default,query_interface,add_ref,release}` | `gzc01_04_framework_identity_and_counts`; unnamed native slots open: GZF1 |
| cGZFramework | §2.2 | service/tick methods, descending stable `add_ordered`, `ordered` | `gzc05_07_services_and_protection`, `gzc39_descending_stable_order`; callback semantic name open: GZF1 |
| cGZFramework | §2.3 | `init`, `main`, `quit`, `abort`, `report_exception`, scheduling/debug getters/setters | `gzc08_10_quit_and_main_failure`, `gzc01_04_framework_identity_and_counts`; partial unwind/loop/platform callback semantics open: GZF1 |
| cGZFramework | §2.4 | borrowed `helper`, `com`, `application`, `set_application`, `unnamed_flag` | `gzc52_borrowed_framework_accessors` |
| cGZFramework | §2.5 | `Streams`, `stream`, `query_stream`, `set_stream`, `propagate_debug_level` | `gzc53_57_lazy_streams_replacement_and_failure_writes`, `gzc58_propagated_debug_is_separate_and_ignores_write_failure` |
| cGZCOM | §3.1 | `Com::query_interface`, `IID`, `framework` | `com_libraries_and_distinct_lookup_paths`; system-service lifetime foreign; loader gaps open: GZF2 |
| cGZCOM | §3.2 | `real_init`, `get_class_object`, `add_library`, `can_unload_library`, `create_guid(s)` | `gzc11_13_com_initialization_and_ids`, `com_libraries_and_distinct_lookup_paths`; loader shutdown/index callback/cache and ID algorithm open: GZF2 |
| cGZCOM | §3.3 | `set_service_running`, `service_running`, `library_array`, `enumerate_libraries` | `gzc59_62_com_running_and_enumeration` |
| cGZCommandServer | §4.1 | `commands::server::{Server,IID,SERVICE_ID}` | `gzc14_17_command_registry_and_defaults`, `gzc63_64_command_faces_and_destructor` |
| cGZCommandServer | §4.2 | `Server` registry/init/shutdown/conversion/execution; existing `StringCommandGenerator`, `Message2CommandDispatcher` | `gzc14_17_command_registry_and_defaults`, `command_dispatch_reentry_and_generator_delegation`; bulk/cancellation/null dispatcher and replacement ownership open: GZF3 |
| cGZCommandServer | §4.3 | QI/shared lifetime/destruction; `execute_formatted`, `formatting::command`; generator replay/null methods | `gzc63_64_command_faces_and_destructor`, `gzc65_66_formatted_command_slot19_bridge`, `gzc67_68_generator_replay_identity_and_null` |
| cGZMiscCommandSystemSupport | §5.1 | `Support::{new,init,shutdown,query_interface,add_ref,release}` | `gzc21_misc_inventory_and_notification_quirk`, `gzc47_49_support_final_release_is_not_shutdown` |
| cGZToolCommandSystemSupport | §5.1 | same `Support`, `Kind::Tool` | `gzc18_20_tool_support_lifetime`, `gzc47_49_support_final_release_is_not_shutdown` |
| cGZMiscCommandSystemSupport | §5.2 | `MISC`, raw duplicate notification list, pending-wait/seed constructor fields | `gzc21_misc_inventory_and_notification_quirk`; `do_message` open: GZF4 (all 35 parameter schemas, IO delegates, waits and errors) |
| cGZToolCommandSystemSupport | §5.3 | `TOOLS`, raw subscription list | `gzc18_20_tool_support_lifetime`; `do_message` open: GZF5 (all seven adapters) |
| cGZBuffer | §6 | storage, QI/lifetime, colour-key getter, mask Unlock, RGB packing | `gzc22_25_buffer_storage_locks_and_colour`, `buffer_formats_and_alloc_failure`, `gzc40_44_buffer_identity_key_and_lifetime`, `gzc45_46_buffer_unlock_mask_and_underflow`; other storage/failure paths open: GZF6 |
| nGZLog::cService | §7.1 | `Log::default`, constants, backend collections | `gzc26_log_level_sentinels`, `log_levels_backend_mutation_and_shutdown`; configuration/full level table open: GZF7 |
| nGZLog::cService | §7.2 | `output` (low-level router), `on_tick`, level setters, Shutdown; first-Init thread capture | GZC27–29 tests, `gzc50_logger_init_thread_owns_delivery`; named/numeric conversion, drain/config edges open: GZF7 |
| nGZLog::cService | §7.3 | `add_output_stream`, borrowed name-set access, public formatted output, `dump_output_levels` | `gzc69_70_output_stream_allocation_and_duplicate_ownership`, `gzc71_73_public_logging_guards_formatting_and_borrowed_sets`, `gzc74_dump_borrows_stream_and_does_not_drain`, `logging_formatter_retry_sizes_and_observed_filtered_category`; full printf vocabulary, low-memory consequences and description table open: GZF7 |
| cGZFileSystem | §8.1 | `FileSystem::default`, constants, cached fields, plugin name and CD option state | `gzc30_31_filesystem_defaults_setter_failure_and_cache`; application components/string lifetime open: GZF8 |
| cGZFileSystem | §8.2 | lazy directory getters; setters preserve cache-valid state on failure; foreign INI/path resolution | `gzc30_31_filesystem_defaults_setter_failure_and_cache`, `gzc51_failed_setters_preserve_valid_cache`; INI literal/path splitting/CD discovery open: GZF8 |
| cGZScriptTerp | §9.1 | `Script::{set_target,set_context,context,push_vocabulary,pop_vocabulary,lookup,dispose}` | `gzc32_33_vocabulary_ownership_shadowing`, `script_setters_release_before_reacquisition`; native constructor/IID/token mutation open: GZF9 |
| cGZScriptTerp | §9.2 | byte tokenizer and parser, direct Host callback protocol | `gzc34_non_uint_translation_is_not_a_command`, `gzc35_36_quoted_value_and_reentry_callbacks`; numeric/unassigned defaults open: GZF9 (injected) |
| cGZScriptTerp | §9.3 | `execute`, `execute_file`, `execute_resource`, error/active reset | `gzc37_unknown_tag_then_text_and_error_reset`, `gzc38_resource_missing_stream_and_release`; final/error callbacks and short reads/NUL/malformed EOF open: GZF9 |
| all nine | §10 | event-recorder tests GZC01–GZC74 (`tests.rs`, `closure_tests.rs`) | all 74 vectors asserted; no game verdict implied |
| all nine | §11 | `Unsupported` boundaries above | GZF1–GZF9 remain spec-open; GZF10/GZF11 closed |

## Fixture policies

* `Script::new_fixture` supplies empty core state; it does not invent the native
  constructor vocabulary. Numeric conversion and unassigned variants are supplied
  by a host resolving GZF9. The parser never decodes XML entities or escapes.
* Buffer allocation obtains arbitrary initialized bytes from an allocator and
  never clears them. It avoids reading uninitialized Rust memory. The public
  drawing `gz_graphics::Buffer` remains a consumer contract, not another storage
  owner. Renderer arithmetic is reused by consumers, not duplicated here.
* Logging uses owned text and Arc backend lifetimes; neither the filter nor output
  callback holds the router lock. The first initialized-host attachment captures
  the delivery thread; repeated attachment does not rebind. Native Init
  configuration stays GZF7; the fixture supplies live sets after Shutdown.
  Reentrant records carry their inherited depth; depth four is dropped. The exact
  native recursive drain sequence remains GZF7.
  Non-sentinel name/value conversions return Unsupported(GZF7), since the missing
  table prevents distinguishing native invalid input from an unrecovered valid
  level. Null/empty names still return -1 and both sentinel pairs are implemented.
  Script inputs containing an embedded NUL return Unsupported(GZF9) before callbacks.
* The default command dispatcher exposes an additive shared-execution entry so
  callbacks can reenter without borrowing the dispatcher mutably. Existing API
  callers retain the original mutable entry. Generator links are weak to avoid
  a Rust allocation cycle; native logical ownership of replacement generators is
  still GZF3. Null dispatch is an explicit checked error in the new public API.

No native ABI exports, game subscriptions, plugin switches or shadow keys are added.

Native printf and text decoding are injected through `formatting::Formatter`.
Core owns limits/retries, guards, status forwarding and the first-word quirk.
`BackendFactory` supplies the output-stream owner's default backend; no timestamp
rendering or level-description table is guessed. Borrowed name sets use scoped
references under the mutex, with stable heap identities and no caller Arc refs.
Tokens model borrowed embedded framework identities by base+offset. Final Release
is terminal; callers cannot reuse destroyed fixtures. Native null dereference or
unbounded null-generator recursion is an explicit Rust precondition panic, never
an invented false-return result.
