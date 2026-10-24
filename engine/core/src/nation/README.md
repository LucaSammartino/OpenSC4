# Nation core

Portable `cSC4Nation` owner from nation.md §§1–8 and demand.md §11.2.
`NationHost` supplies platform/game interfaces; `NationRegion` supplies region
operations, including the region factory (§6), and `TutorialCity` supplies the
regional-city interface. Returned region/city handles are owned and release on
Drop. Getters borrow them without taking or releasing COM references. Getters
use immutable shared state; no RefCell is involved. This object has no city save
record, so no codec is applicable.

Use one `ProcessDirectories` per process (or `ProcessDirectories::global()`). Its
OnceLock stores the two directory strings and consumes StaticInit even without
an app. Tuning belongs to each nation, is destroyed at Shutdown, and is reread at
the next Init. Difficulty survives Shutdown and resize-to-three retains existing
elements when an explicitly empty ID list skips difficulty loading.

## Owner closure

| Owner / nation section | Implementation | Coverage |
|---|---|---|
| cSC4Nation §1 application state, no serialization | `Nation`, owned regions, `ProcessDirectories` | `construction_selection_and_failures`, `lifecycle_rereads_tuning_releases_catalog_and_guards_only_directories`; no save record |
| cSC4Nation §2 / demand §11.2 economy and defaults | `EconomyTuning`, `EconomyDate`, `get_economy_modifier` | `nation_section5_vector1_shipped_difficulty_and_no_city`, `nation_section5_vector2_month_before_start_is_unsigned` |
| cSC4Nation §2.1 malformed tuning | `EconomyTuning::get_economy_modifier` safety errors | `nq3_numeric_and_error_vectors` |
| cSC4Nation §3 messages | `do_message`, `init`, `shutdown` | `preference_uses_display_bytes_and_sixty_four_byte_prefix_without_probing`, `tutorial_retention_exports_signed_positions_failures_and_screen_lifecycle`, lifecycle test |
| cSC4Nation §4 interface operations and ownership | `Nation` methods, borrowed getters and owned handles/Drop; binary slot layout belongs to the adapter boundary | `construction_selection_and_failures`, `deletion_order_neighbors_last_region_and_null_creation`, lifecycle test; no native adapter in this core module |
| cSC4Nation §5 conformance | `EconomyTuning`, `EconomyDate` | Both `nation_section5_vector*` tests in `engine/conformance/tests/nation.rs` |
| cSC4Nation §5.1 owner-contract conformance | `Nation` lifecycle, catalog and tutorial operations | `engine/conformance/tests/nation_owner.rs` owner vectors, including `nq3_numeric_and_error_vectors` |
| cSC4Nation §5.2 directory-case conformance | `lower_directory`, `init`, `reset_tutorial_cities`, preference lookup | All ten `nq4_vector1_*` through `nq4_vector10_*` tests in `nation_owner.rs` |
| cSC4Nation §6.1 discovery and copying | `discover_regions`, `copy_region_files`, `copy_new_user_regions`, `ProcessDirectories` | `discovery_preference_and_probe_selection`, `template_copy_order_paths_overwrite_and_starter_fallback`, `recursive_copy_creates_parents_and_continues_after_ignored_results`, lifecycle test |
| cSC4Nation §6.2 initialization/preferences | `init`, `get_index_of_current_region_setting_in_preferences` | `discovery_preference_and_probe_selection`, `preference_uses_display_bytes_and_sixty_four_byte_prefix_without_probing`, `nq4_vector7_*`, `nq4_vector9_*`, `nq4_vector10_*` |
| cSC4Nation §6.3 cleanup/reinit | `shutdown`, `EconomyTuning::reload` | `lifecycle_rereads_tuning_releases_catalog_and_guards_only_directories` |
| cSC4Nation §6.4 UTF-8 and CRT case mapping | `lower_directory`, `NationHost::lowercase_narrow/lowercase_wide`; registry values compared verbatim | `nq4_vector1_*` through `nq4_vector10_*`, `nq4_del_threshold_wide_prefix_and_failed_mapping`, `nq4_reset_registry_key_is_verbatim_even_after_locale_change` |
| cSC4Nation §7 catalog getters/selection/add/delete | Borrowed getters, `set_active_region`, `add_new_region`, `delete_region` | `construction_selection_and_failures`, `deletion_order_neighbors_last_region_and_null_creation`, `deleting_last_catalog_entry_selects_previous_before_compaction` |
| cSC4Nation §8 tutorial reset | `reset_tutorial_cities` | `tutorial_retention_exports_signed_positions_failures_and_screen_lifecycle`, `tutorial_recreation_default_first_and_early_failure_keep_partial_effects`, `nq4_vector8_*`, `nq4_reset_registry_key_is_verbatim_even_after_locale_change` |

NQ1–NQ4 are answered and implemented. All specified portable cSC4Nation owner
behavior is implemented; no owner sections remain open or Unsupported. The
active CRT LC_CTYPE map is an explicit `NationHost` dependency, independent of
the registry's language/default selection. `lower_directory` implements the
legacy UTF-8-to-word transform, including prefix truncation on malformed input
and low-16-bit truncation of four-byte characters. It neither normalizes nor
expands case mappings. Host adapters retain words when the Windows map fails.
Replacement safety errors are distinct from original bool/void
returns; ordinary region, copy, export and move failures never trigger rollback.
Region factories and regional-city parsing remain the responsibility of their
respective interfaces, rather than duplicated nation algorithms.

## Integration boundary

This module has no C ABI or GZCOM adapter and is not installed as a replacement
or shadow in the game. A future adapter must implement the interfaces using the
game's services, wire the process directory owner, and compare behavior in-game.
Synthetic conformance is measured here; game shadow/ref-warning results are not.
