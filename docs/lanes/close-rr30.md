| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-rr30 region/regional-city closure; engine | Codex / .claude/worktrees/close-rr30 / reserved close-rr30 (detached HEAD) | ready; code commits 4742079c9, 5e2cbf098, 9cb30301c, 6565a189e; ABI exports unchanged; no seam reservation; no new questions | 1. Supervisor publish detached commits. 2. Land engine queue tip. |

- Closure tables: region 51 implemented / 5 open / 2 external; regional_city 30 implemented / 2 open / 1 external (their README Status columns). Every implemented row cites code and tests.
- Final gate: qbuild all PASS; Rust 7969/7969 (`build-runs/qbuild-test.log`), clippy 0 diagnostics (`qbuild-clippy.log`), native build 31/31 edges (`fc4verify-build.log`), CTest 129/129 (`fc4verify-ctest.log`).
- Focused checks: region 27/27, regional_city 22/22, regional conformance 4/4; fmt 8 owned Rust files, zero differences. All named closure tests also pass in final qbuild-test.log.
- region-model §3.1: RM15 applied in localization_id; rm15_name_token_worked_cases covers all published tokens.
- region-model §§5.5,7.1: RM17 applied in box/reload handling; reload_success_failure_and_delete_city covers stale aliases, counted survival, replacement and retained files.
- region-model §6.4: RM5 UTF-8, RM6 starter and RQ3 suffixes covered by create_sanitization_templates_starter_and_failure_paths.
- region-model §7.3 and region §6: RQ3 result matrix covered by move_failures_always_reinit_and_same_region_reassigns_serial and existing collision test.
- region-model §7.4 and region §10: RM18/RQ2 filename rule covered by rm18_truncated_filename_worked_cases and filename_conformance_and_serial_generation; dotless collisions report FilenamePosition.
- region-model §7.6: RM17/RQ3 reset failure/polarity covered by tutorial_reset_polarity_reload_failure_and_reactivation; §8.1 and region §7: RM16 covered by terrain_height_table_low_water_bytes_and_thread_safe_getters.
- regional-city §2/§7.1 and records §§5.1–5.3: RY2/RR7/RR8 preserved strings, metadata and immediate preferences path covered by ry2_rr7_rr8_metadata_preferences_and_opaque_string_preservation.
- records §§2.1–2.2: RR9 prefix, unrelated payload and cleanup result covered by rr9_cleanup_preserves_result_and_unrelated_record; §§4.1–4.4: RR4/RR5 covered by rr4_rr5_west_highway_orientation_singleton_and_pair.
- records §§4.5–4.8: RR3 corrected last grouped mask/border byte and native slot/query/tool requests; rr3_last_group_mask_and_city_border_byte plus regionalcity_border_contract and abi_conformance_test pass.
- records §5.4: RR6 stored-float reference covered by rr6_workforce_getter_returns_address_of_stored_float; foundations §§11.1–11.2: RY7 covered by three ry7_* tests and corrected foundations_record_and_deal_lifecycle.
- Open region rows: model §7.3 RM7 needs same-region serial/rename observation; §7.6 RM8 nation's bool use; §12.2 RM12 first-load order/tutorial rebuild; §13 retains those and RM9 tuning values/height-water consumers; region §10 RQ1 attribute-2/effective writability.
- Open regional rows: regional-city §9 RY2 needs nonempty original string producer/writer stack; records §9 retains RR4 property-only marker/vehicle comparison, RR5 stock singleton producer/rendering, RR6 return consumers, RR7 metadata writer stacks, RR8 preference/path/disk trace, RR9 physical allocation trace.
- Game close-rr30-shadow-20261026-2: clean, ref_warnings=0, end 2026-03-01, 11.33 days/s; 3/3 exact samples, 85/85 records each, 48279/48279 fields, zero resyncs. Evidence: C:/Users/tlori/FreeCity4/build-runs/game-results/close-rr30-shadow-20261026-2.log and .verdict.
- Distinct failed attempts: first gate's old commodity assertion/ABI count retained in build-runs/qbuild-first-*.log, then fixed; game label -1 rejected before launch for request-level close_dialogs, corrected under [ini] in -2. Turbo warning is explained by disabled replacement simulator/native clock.
- Limits: private C API wire transport remains an atomic factory decoder; core in-place Read retains partial mutations. Native workforce getter's existing thread-local snapshot does not establish stored-member address identity; this lane tests the core address contract only.
