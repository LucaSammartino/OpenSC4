# MySims core

Behavior source: `docs/specs/mysims.md`. Existing MySim, advisor and street
records are reused. `dispatch.rs` owns persistent job-search state and
`pool.rs` owns template loading/import. Host traits supply external services;
opaque occupant IDs carry identity, while native reference ownership remains
the adapter's responsibility. These Rc-based core hosts are single-threaded;
no plugin or C ABI integration is provided here.

## Owner closure

One row per applicable specification section and owner. Rows marked open are
explicit boundaries, not a claim that the complete native owner is replaced.
The spec calls the dispatch automaton out of scope in section 1.1 and permits
a stub in section 10. Its unpublished behaviors and record stay **Unsupported**
under MS17. The pool has no remaining open section.

### cSC4MySimDispatch

| Spec section | Code item or open contract | Coverage |
|---|---|---|
| 1.1 identity/scope | `MySimDispatch::{class_id,interface_id}`; automaton open: MS17 | `t39_ids` |
| 2.2 dispatch state read by MySim | `DispatchLocation`, `JobSearchState`, `get_state` | `t40_location_contract`, `t41_set_state` |
| 3.3 notifications | `set_state`, `report_flags`, `report_arrival`, `report_sim_state`; autonomous trip notifications open: MS17 | `t38_notifications`, `t41_set_state` |
| 4.1 tracking flag/home | `check_flags`, `set_default_destination` | `t38_notifications`, `t36_dispatch_find_new_job` |
| 4.6 location copies and trip-state input | `DispatchLocation::set_cell_position`, `get_state`; location/wait evolution open: MS17; Lua refresh belongs to MySim | `t42_ms13_arithmetic_and_home_rect`, `ms12_29_location_arithmetic` |
| 4.7 callers | `find_new_job`, `change_job`, `job_destination`, `shutdown` | `t36_dispatch_find_new_job`, `t37_default_destination_at_state_2` |
| 4.7.1 location, validation, path reset/search, accumulation, notifications | `world_to_cell`, `DispatchLocation`, `reset_path`, `validate_commute_location`, `set_current_commute_destination`, `home_start_rect` | `t30`-`t35`, `t40`-`t42`; vectors `ms12_21`-`ms12_25`, `ms12_28`, `ms12_29` |
| 4.7.2 persistent search versus fresh CheckCommute factory | `init`, persistent `pathfinders`; CheckCommute belongs to MySim | `t36_dispatch_find_new_job` (persistent wealth flags) |
| 4.11 dispatch tuning effects | open: MS17 (values are named; timing/transition algorithm is outside this spec) | Unsupported |
| 6.2 dispatch ref/inline record | open: MS17 (MySim record owns the ref, dispatch's own bytes unpublished) | Unsupported |
| 7.2 dispatch trip/UI entry points | open: MS17 (LeaveForWork/ReturnHome and U-Drive-It delegated to out-of-scope automaton) | Unsupported |
| 8 dispatch RNG | open: MS17 (explicitly out of scope) | Unsupported |
| 9.1 path queries do not apply traffic | `validate_commute_location`, `CoreJobPathfinder` query-only interface | `t30_job_search_setup` exact call sequence; `ms12_20_no_feedback` |
| 9.2 pathfinder/lot/neighbour dependencies | `JobPathfinder`, `JobSearchWorld`, `DispatchHost` | `t30`-`t35`, `t40_location_contract`; `ms12_21`-`ms12_25`, `ms12_28` |
| 10 replacement boundary | Core job search implemented; trip automaton open: MS17 | No native replacement or ABI change |
| 12.6 job search/notification | `find_new_job`, `set_current_commute_destination` | `ms12_06_job_search`, `ms12_21`-`ms12_25` |
| 12.21 MS13 arithmetic and home rectangle | `world_to_cell`, `set_cell_position`, `home_start_rect` | `t42_ms13_arithmetic_and_home_rect`, `ms12_29_location_arithmetic`, `ms12_31_home_rect` |
| 12.23-26 MySim commute/trip sampling | MySim-owned, not assigned to dispatch; persistent job-search configuration kept separate | No closure claim for the MySim owner |

### cSC4MySimPool

| Spec section | Code item | Coverage |
|---|---|---|
| 1.1 app/UI template owner | `MySimPool`, `MySimTemplate`, `PoolHost` | `p4_get_templates` |
| 4.12 built-in enumeration, lazy cache, output replacement | `get_mysim_templates`, `enum_keys_callback` | `p4_get_templates`, `p10_ms12_absent_properties`, `ms12_26_pool_records` |
| 4.12 import record reader/writer, portraits and byte paths | `parse_mysims_dat`, `encode_record`, `portrait_source`, `data_file_path`, `userdata_dir` | `p1_record`, `p2_display_path`, `p8_ms12_paths_and_lengths`, `ms12_26_pool_records` |
| 4.12 installation and import lifecycle/cache | `find_the_sims_installation`, `import`, `import_family` | `p5_import`, `p6_find_installation`, `p7_cache`, `ms12_27_pool_import_cache` |
| 4.12 ini reads, required texts, localized comparisons, unsigned/signed formatting | `IniFile`, `import_family`, `member_indices`, `member_key`, `member_face`, `is_male_text`, `make_lower` | `p3_gender`, `p9_ms12_ini_reads`, `p11_ms14_required_reads_and_short_names`, `ms12_30_import_branches` |
| 12.22 MS14 branches | `import_family`, `member_key`, `member_face` | `p11_ms14_required_reads_and_short_names`, `ms12_30_import_branches` |

`n = UINT_MAX` faithfully uses a wrapping, nonterminating member loop; tests
exercise signed formatting directly and finite negative bounds instead. Missing
built-in scalar properties use the spec's defined 0/false defaults and retain
indeterminate-property markers. No game assets or platform services are embedded.
