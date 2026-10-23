# Portable application directors

Source: app-directors.md §§1–9 and its explicit registration references. This
module owns the four directors' state, shared algorithms and bootstrap calls.
`game_data_registry::Registry` remains the single registry implementation.
No plugin/capi binding or ABI layout is introduced. Tokens identify borrowed
foreign objects unless a COM/QI/manager output explicitly supplies a reference.
Those local references are released after registration; child/COM/singleton
tokens are never retained or released. Getters use shared reads without RefCell.

`Director::construct_game` takes the result of the global root lookup, then
captures global COM once and applies the owned `add_director` algorithm to that
root. Its own children stay empty. Subordinate receivers remain foreign interfaces.
`Singleton::get` is the ordinary getter; `begin_get`/`complete_get` also expose
the set-before-construction phase for recursive adapters. Complete only when
`begin_get` requested construction. Exit destroys base state without freeing
static storage. Heap fixture destruction rejects freeing static storage.

Foreign factories return their actual interface identities, including adjusted
faces. Registered-object algorithms (decoder, editors, persistence, drawing and
IO) are separate owners: ADIR3 does not authorize inventing their interfaces.
`Boundary` reports invalid native preconditions, including a null kind-1 factory
object, rather than pretending native dispatch gracefully returns false.

## Owner closure

Test names below are in `tests.rs` unless prefixed `registry::`, which refers to
`game_data_registry/tests.rs`. Shared rows are deliberately repeated per owner.
Enumeration verifies class sets and child/local phase order. Ascending local
class order is a portable choice; exact native hash order remains open ADIR1.

| Owner | Spec section | Code item | Test / open contract |
|---|---|---|---|
| Tools | §1 boundary | `Host`, `Owner::Tools` | `ad_v12_all_valid_noop_hooks_and_start` |
| Tools | §2 base state | `Director::new` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| Tools | §2.1 identity, count, getters, hooks | `query_interface`, `add_ref`, `release`, `hook`, `library_path` | `ad_v1_v2_v3_shared_faces_and_non_deleting_lifetime`, `ad_v12_all_valid_noop_hooks_and_start`, `ad_v25_library_path_heap_size_and_owner_ids` |
| Tools | §2.2 COM and children | `initialize`, `add_director` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| Tools | §2.3 lookup and enumeration | `add_class`, `get_class_object`, `enum_classes` | `ad_v7_v8_child_first_lookup_and_preserved_outputs`, `ad_v9_factory_qi_failure_success_callback_and_null_fault`, `ad_v11_enumeration_children_then_local_duplicates_and_false_callbacks`; open: ADIR1 (exact local order) |
| Tools | §2.4 unload and destruction | `can_unload`, `destroy` | `ad_v10_unload_count_short_circuits_and_children_stop_at_false`, `ad_v27_deleting_flags_free_only_low_bit_never_release_borrowed_refs` |
| Tools | §3 construction and no-op startup | `TOOLS_CLASSES`, `start`, `Singleton` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces`, `ad_v12_all_valid_noop_hooks_and_start`, `ad_v26_getters_borrow_static_identity_once_and_allow_recursive_access`, `tools_unwind_retry_and_other_getter_unwind_open_adir2` |
| ResourceLib | §1 boundary | `Host`, `Owner::ResourceLib` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces` |
| ResourceLib | §2 base state | `Director::new` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| ResourceLib | §2.1 identity, count, getters, hooks | `query_interface`, `add_ref`, `release`, `hook`, `library_path` | `ad_v1_v2_v3_shared_faces_and_non_deleting_lifetime`, `ad_v12_all_valid_noop_hooks_and_start`, `ad_v25_library_path_heap_size_and_owner_ids` |
| ResourceLib | §2.2 COM and children | `initialize`, `add_director` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| ResourceLib | §2.3 lookup and enumeration | `add_class`, `get_class_object`, `enum_classes` | `ad_v7_v8_child_first_lookup_and_preserved_outputs`, `ad_v9_factory_qi_failure_success_callback_and_null_fault`, `ad_v11_enumeration_children_then_local_duplicates_and_false_callbacks`; open: ADIR1 (exact local order) |
| ResourceLib | §2.4 unload and destruction | `can_unload`, `destroy` | `ad_v10_unload_count_short_circuits_and_children_stop_at_false`, `ad_v27_deleting_flags_free_only_low_bit_never_release_borrowed_refs` |
| ResourceLib | §5.1 construction/getter/factories | `RESOURCE_CLASSES`, `Singleton` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces`, `ad_v26_getters_borrow_static_identity_once_and_allow_recursive_access`; open: ADIR2 (exception retry), ADIR3 (factory-owned interfaces) |
| ResourceLib | §5.2 Start | `start` | `ad_v13_v14_resource_start_stored_com_pointer_test_and_repeat` |
| ResourceLib | §5.3 resource binding and dispatcher | `hook`, `dispatch_hooks` (enabled phase only) | `ad_v15_v16_resource_binding_missing_manager_and_false_results`, `ad_v17_false_hook_does_not_cancel_framework_phase` |
| ResourceLib | §5.4 drawing/IO boundary | `Host::factory`, `Host::query` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces`; open: ADIR3 (created-object semantics) |
| ResMan | §1 boundary | `Host`, `Owner::ResMan` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces` |
| ResMan | §2 base state | `Director::new` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| ResMan | §2.1 identity, count, getters, hooks | `query_interface`, `add_ref`, `release`, `hook`, `library_path` | `ad_v1_v2_v3_shared_faces_and_non_deleting_lifetime`, `ad_v12_all_valid_noop_hooks_and_start`, `ad_v25_library_path_heap_size_and_owner_ids` |
| ResMan | §2.2 COM and children | `initialize`, `add_director` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| ResMan | §2.3 lookup and enumeration | `add_class`, `get_class_object`, `enum_classes` | `ad_v7_v8_child_first_lookup_and_preserved_outputs`, `ad_v9_factory_qi_failure_success_callback_and_null_fault`, `ad_v11_enumeration_children_then_local_duplicates_and_false_callbacks`; open: ADIR1 (exact local order) |
| ResMan | §2.4 unload and destruction | `can_unload`, `destroy` | `ad_v10_unload_count_short_circuits_and_children_stop_at_false`, `ad_v27_deleting_flags_free_only_low_bit_never_release_borrowed_refs` |
| ResMan | §6.1 construction/getter/factories | `RESMAN_CLASSES`, `Singleton` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces`, `ad_v26_getters_borrow_static_identity_once_and_allow_recursive_access`; open: ADIR2 (exception retry), ADIR3 (created-object interfaces) |
| ResMan | §6.2 Start and failures | `start` | `ad_v18_v19_resman_supplied_com_and_null_service` |
| Game | §1 boundary | `Host`, `Owner::Game` | `ad_v20_v21_game_wires_global_root_fourteen_in_order_ten_starts` |
| Game | §2 base state | `Director::new` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| Game | §2.1 identity, count, getters, hooks | `query_interface`, `add_ref`, `release`, `hook`, `library_path` | `ad_v1_v2_v3_shared_faces_and_non_deleting_lifetime`, `ad_v12_all_valid_noop_hooks_and_start`, `ad_v25_library_path_heap_size_and_owner_ids` |
| Game | §2.2 COM and children | `initialize`, `add_director` | `ad_v4_v5_v6_initialization_and_descendant_first_flattening` |
| Game | §2.3 lookup and enumeration | `add_class`, `get_class_object`, `enum_classes` | `ad_v7_v8_child_first_lookup_and_preserved_outputs`, `ad_v9_factory_qi_failure_success_callback_and_null_fault`, `ad_v11_enumeration_children_then_local_duplicates_and_false_callbacks`; open: ADIR1 (exact local order) |
| Game | §2.4 unload and destruction | `can_unload`, `destroy` | `ad_v10_unload_count_short_circuits_and_children_stop_at_false`, `ad_v27_deleting_flags_free_only_low_bit_never_release_borrowed_refs` |
| Game | §7.1 singleton and bootstrap | `Singleton`, `construct_game`, `SUBORDINATES` | `ad_v20_v21_game_wires_global_root_fourteen_in_order_ten_starts`, `ad_v26_getters_borrow_static_identity_once_and_allow_recursive_access`; open: ADIR2 (exception retry) |
| Game | §7.2 and Appendix A classes | `GAME_CLASSES`, `get_class_object` | `ad_v22_complete_class_sets_dispatch_foreign_qi_faces`, `registration_inventories_match_published_spec_independently`; open: ADIR3 (created-object algorithms) |
| Registry | §1 and §4 identity/construction | `Registry::new`, metadata getters, `query_interface_out` | `registry::ad_v23_registry_qi_clears_sentinel_unlike_director`, `registry::g14_service_shared_count_views_and_zero_one_greater_release` |
| Registry | §4 entries and callback boundary | existing `Registry` entry/traversal/subscription methods | registry G1–G11/G13 tests; open: GDR1–GDR4 (linked contract), GDR5 (anonymous collision) |
| Registry | §4 service init/shutdown | `Registry::init`, `Registry::shutdown` | `registry::g12_shutdown_preserves_root_record_but_not_membership` (AD-V24) |
| Tools | §8 vectors AD-V1–12,25–27 | above methods | `ad_v*` tests |
| ResourceLib | §8 vectors AD-V1–17,22,25–27 | above methods | `ad_v*` tests |
| ResMan | §8 vectors AD-V1–12,18–19,22,25–27 | above methods | `ad_v*` tests |
| Game | §8 vectors AD-V1–12,20–21,25–27 | above methods | `ad_v*` tests |
| Registry | §8 vectors AD-V23–24 | existing registry plus QI wrapper | `registry::ad_v23_registry_qi_clears_sentinel_unlike_director`, `registry::g12_shutdown_preserves_root_record_but_not_membership` |
| Tools / ResourceLib / ResMan / Game / Registry | §9 open boundaries | explicit boundaries above | ADIR1–3 and linked owner questions remain open; no new gaps inferred |

No real-game replacement or shadow is claimed: this is a core-only model with
foreign interfaces and literal conformance fixtures. The source tests include
the published Appendix A independently to catch class/factory transcription errors.
