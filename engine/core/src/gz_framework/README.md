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
| cGZFramework | §2.1 | `Framework::{default,query_interface,add_ref,release}` | `gzc01_04_framework_identity_and_counts`; unnamed native slots open: GZF1; accessors open: GZF10 |
| cGZFramework | §2.2 | `add_service`, `lookup`, `remove_service`, tick/ordered registry methods, `services` | `gzc05_07_services_and_protection`; callback kind/ties open: GZF1; order direction choice: GZF10 |
| cGZFramework | §2.3 | `init`, `main`, `quit`, `abort`, `report_exception`, scheduling/debug getters/setters | `gzc08_10_quit_and_main_failure`, `gzc01_04_framework_identity_and_counts`; partial unwind/loop/platform callback semantics open: GZF1 |
| cGZCOM | §3.1 | `Com::query_interface`, `IID`, `framework` | `com_libraries_and_distinct_lookup_paths`; system-service lifetime foreign, remaining inventory open: GZF2/GZF11 |
| cGZCOM | §3.2 | `real_init`, `get_class_object`, `add_library`, `can_unload_library`, `create_guid(s)` | `gzc11_13_com_initialization_and_ids`, `com_libraries_and_distinct_lookup_paths`; loader shutdown/index callback/cache and ID algorithm open: GZF2 |
| cGZCommandServer | §4.1 | `commands::server::{Server,IID,SERVICE_ID}` | `gzc14_17_command_registry_and_defaults`; inherited system-service QI/lifetime foreign, inventory-only contracts open: GZF11 |
| cGZCommandServer | §4.2 | `Server` registry/init/shutdown/conversion/execution; existing `StringCommandGenerator`, `Message2CommandDispatcher` | `gzc14_17_command_registry_and_defaults`, `command_dispatch_reentry_and_generator_delegation`; bulk/cancellation/null dispatcher and replacement ownership open: GZF3 |
| cGZMiscCommandSystemSupport | §5.1 | `Support::{new,init,shutdown,add_ref,release}` | `gzc21_misc_inventory_and_notification_quirk`; QI/final-release open: GZF10 |
| cGZToolCommandSystemSupport | §5.1 | same `Support`, `Kind::Tool` | `gzc18_20_tool_support_lifetime`; QI/final-release open: GZF10 |
| cGZMiscCommandSystemSupport | §5.2 | `MISC`, raw duplicate notification list, pending-wait/seed constructor fields | `gzc21_misc_inventory_and_notification_quirk`; `do_message` open: GZF4 (all 35 parameter schemas, IO delegates, waits and errors) |
| cGZToolCommandSystemSupport | §5.3 | `TOOLS`, raw subscription list | `gzc18_20_tool_support_lifetime`; `do_message` open: GZF5 (all seven adapters) |
| cGZBuffer | §6 | `Buffer` allocation, dimensions/rect/type/bpp/stride/pixels, lock/unlock, RGB packing | `gzc22_25_buffer_storage_locks_and_colour`, `buffer_formats_and_alloc_failure`; QI/color key defaults/lifetime open: GZF10; resize/attachment/palette/conversion and allocation-failure aftermath open: GZF6; Unlock conflict: GZF10 |
| nGZLog::cService | §7.1 | `Log::default`, service/IID constants, backend collection methods | `gzc26_log_level_sentinels`, `log_levels_backend_mutation_and_shutdown`; configuration/full level table/stream inventory open: GZF7/GZF11 |
| nGZLog::cService | §7.2 | `output`, `on_tick`, `set_output_level`, `shutdown`, sentinel conversion helpers | `gzc27_log_filter_decline`, `gzc28_log_recursion_deferred_and_bounded`, `gzc29_foreign_thread_fifo_owns_text`, `log_levels_backend_mutation_and_shutdown`; named/numeric conversions and drain/config edge cases open: GZF7; owner-thread choice: GZF10 |
| cGZFileSystem | §8.1 | `FileSystem::default`, constants, cached fields, plugin name and CD option state | `gzc30_31_filesystem_defaults_setter_failure_and_cache`; application components/string lifetime open: GZF8 |
| cGZFileSystem | §8.2 | lazy `data_directory`, `plugin_directory`, non-atomic setters; foreign INI/path resolution | `gzc30_31_filesystem_defaults_setter_failure_and_cache`; INI literal/path splitting/CD discovery open: GZF8; cache invalidation choice: GZF10 |
| cGZScriptTerp | §9.1 | `Script::{set_target,set_context,context,push_vocabulary,pop_vocabulary,lookup,dispose}` | `gzc32_33_vocabulary_ownership_shadowing`, `script_setters_release_before_reacquisition`; native constructor/IID/token mutation open: GZF9 |
| cGZScriptTerp | §9.2 | byte tokenizer and parser, direct Host callback protocol | `gzc34_non_uint_translation_is_not_a_command`, `gzc35_36_quoted_value_and_reentry_callbacks`; numeric/unassigned defaults open: GZF9 (injected) |
| cGZScriptTerp | §9.3 | `execute`, `execute_file`, `execute_resource`, error/active reset | `gzc37_unknown_tag_then_text_and_error_reset`, `gzc38_resource_missing_stream_and_release`; final/error callbacks and short reads/NUL/malformed EOF open: GZF9 |
| all nine | §10 | event-recorder tests GZC01–GZC38 | all 38 vectors asserted; no game verdict implied |
| all nine | §11 | `Unsupported` boundaries above | GZF1–GZF11 remain open |

## Fixture policies

* `Script::new_fixture` supplies empty core state; it does not invent the native
  constructor vocabulary. Numeric conversion and unassigned variants are supplied
  by a host resolving GZF9. The parser never decodes XML entities or escapes.
* Buffer allocation obtains arbitrary initialized bytes from an allocator and
  never clears them. It avoids reading uninitialized Rust memory. The public
  drawing `gz_graphics::Buffer` remains a consumer contract, not another storage
  owner. Renderer arithmetic is reused by consumers, not duplicated here.
* Logging uses owned text and Arc backend lifetimes; neither the filter nor output
  callback holds the router lock. The construction thread is the fixture owner.
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
