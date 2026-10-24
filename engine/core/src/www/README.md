# Portable web owners

Behavior source: `docs/specs/web-network.md` and its linked FTP/lifetime contracts.
The existing `rzurl` module remains the sole URL parser. No ABI/plugin integration,
OS filesystem, socket transport, or live game equivalence is claimed here. `Host`,
`Io` and `Runtime` supply foreign IO, clock, allocation identities and runtime
calendar conversion. Mutable operations require exclusive Rust access; getters
have no `RefCell`. Native references are explicit effects, except cache RAM buffers
whose portable ownership uses `Arc`; no-copy insertion transfers a `Vec` allocation.
Adapters must supply native allocator compatibility and stay within the specified
16-bit reference/32-bit size domain. Constructor path literals not qualified by the
spec are explicit inputs, rather than invented defaults.

## Owner closure

Each row identifies executable coverage of the published contract. An `open:` suffix
is an existing spec limit, not a successful native false return. Rows do not claim
closure of the parked whole-owner address inventory. The literal choices in Q15
are implemented and tested, pending spec clarification.

| Owner / spec section | Code item | Test / remaining boundary |
|---|---|---|
| CookieManager 3.1 record/defaults | `cookies::Cookie` | `parse_defaults_attributes_and_literal_port_rule` |
| CookieManager 3.2 parser | `Cookie::parse`, shared `ftp::atol` | `parse_defaults_attributes_and_literal_port_rule`; literal n-1 ports, open: Q1; splitter edge choices Q15 |
| CookieManager 3.3 admission/replacement | `Manager::add`, `add_text` | `admission_replacement_expiration_and_lookup` |
| CookieManager 3.4 lookup/header | `lookup`, `lookup_text`, `lookup_into`, `header`, `header_text` | `header_version_quirk_and_eviction_order`, `admission_replacement_expiration_and_lookup`, `copying_lookup_does_not_alias_store_and_miss_preserves_output`; literal pair-index choice Q15; open: Q16 miss-side output |
| CookieManager 3.5 eviction | `Manager::evict` | `header_version_quirk_and_eviction_order` |
| CookieManager 3.6 lifecycle/persistence/settings | `Manager::{new,init,shutdown,read,write,clear_file,enumerate,set_binary,set_file_path}` | `ck01_through_ck06_and_lifecycle`, `persistence_read_cap_and_init_nonempty_guard`; open: Q3 binary/character conversion, unqualified path literal; settings choices Q15 |
| CookieManager 3.7 inventory/vectors | portable methods above (native QI/refcounts are adapter roles) | `ck01_through_ck06_and_lifecycle`: CK01–CK06 |
| WWWSystem 4.1 identity/lifecycle | `service::Www::{construct_with_host,query_interface,post_init,shutdown,destroy}` | `identity_default_hold_order_and_dispatch_target_selection`, `constructed_components_and_ftp_admission_vectors`, `progress_borrowed_no_waiter_update_and_cancel_all`; open: Q7/Q9 residual attribution inventory |
| WWWSystem 4.2 dispatch/holds | `Www::{load_text,load,set_default_target}` | `handler_failure_owns_attempt_and_releases_after_callback`, `identity_default_hold_order_and_dispatch_target_selection` |
| WWWSystem 4.3 local files | `Www::local` | `local_threshold_open_fallback_and_read_failure_nonzero_id` |
| WWWSystem 4.4 HTTP/cache/duplicates | `Www::admit_http` | `cache_threshold_and_failed_retrieval_continues_to_admission`, `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry`, `http_admission_helper_spans_submission_and_www_insertion`; open: Q9 opaque Cookie-header record construction |
| WWWSystem 4.5 status/cancel | `Www::{status,get_job,http_id,ftp_id,cancel,cancel_all}` | `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry`, `progress_borrowed_no_waiter_update_and_cancel_all` |
| WWWSystem 4.6 messages/progress | `Www::{envelope,progress,do_message}` | `progress_borrowed_no_waiter_update_and_cancel_all`, `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry` |
| WWWSystem 4.7 completion/fanout | `Www::complete_http`, `convert` | `completion_conversion_failure_cookies_and_cache_before_target`, `cancellation_late_resource_and_foreign_admission_zero_hold`; open: Q10 matched null-target suffix |
| WWWSystem 4.8 FTP call boundary | `Www::{load,complete_ftp}` | `constructed_components_and_ftp_admission_vectors`; open: Q7 foreign scheduler implementation |
| WWWSystem 4.9 settings/handlers/visited | `Www` settings/handler methods, `http::Client::{set_directory,set_cache_setting}`, `path_preference` | `splitter_empty_slots_quote_and_directory_side_effects`, `http_directory_and_cache_setting_are_independent_copies`, `handler_failure_owns_attempt_and_releases_after_callback`; implemented, including embedded HTTP state after failed cache directory creation |
| WWWSystem 4.10 WWW-01–WWW-20 vectors | service test module | named tests in preceding rows and vector index below |
| WWWSystem 4.11 path conversion | `www::{native_path,percent_decode}` | `published_native_path_vectors` |
| WWWSystem 4.12 response cache policy | `service::{cache_tokens,cache_policy}` | `policy_distinguishing_headers_and_wrapping_dates`, `splitter_empty_slots_quote_and_directory_side_effects` |
| WWWSystem 4.13 reached FTP bodies | existing `ftp`, `listing`, `worker` | `constructed_selector_vectors`, `retr_zero_data_final_reply_and_paths`, `listing_partial_capacity_and_replacement_suffix`, `outer_worker_suspend_stop_cancel_and_nonftp_preserved_status`, `shutdown_release_before_disconnect_uninitialized_and_forced_work`; open: Q7 remaining commands, admission and support bodies |
| WWWSystem 4.14 reached HTTP admission | `http::{Client,Request,Job,method_id,method_name}` | `method_prefix_and_wrapper_boundaries`, `admission_wrap_raw_cancel_leaks_and_first_match_wins`, `zero_and_one_request_ref_delete`; open: Q9 request/first-record normalization |
| HTTPClient 5.1 construction/lifetime/calendar | `Client::{new,complete,final_boundary,shutdown,worker_exit}`, `notification::time_to_seconds` | `cancellation_callback_before_release_and_stop_leaks`, `shutdown_replacement_then_raw_drain_no_join`, `calendar_offsets_and_runtime_adjustment`; open: Q13 date grammar, wire/framing/finalization and residual slots, Q9 normalization |
| HTTPClient 5.2 synchronous notifications | `Client::send_notification`, completion effects | `http01_through_http09`, `cancellation_callback_before_release_and_stop_leaks` |
| HTTPClient 5.3 progress | `Client::update` | `http01_through_http09` |
| HTTPClient 5.4 HTTP01–HTTP09 vectors | notification test module | `http01_through_http09` |
| NetworkFileCache 6.1 defaults/settings/inventory | `cache::{Cache,Entry,Data}`, numeric setters | `holders_loading_failures_paths_settings_and_persistence`, `nfc01_through_nfc06`; open: Q11 constructor INI literal and filename allocation |
| NetworkFileCache 6.2 lifecycle/paths | `Cache::{init,shutdown,clear_map,directory,ini_name,set_directory,set_ini_name}` | `nfc07_through_nfc12`, `holders_loading_failures_paths_settings_and_persistence`, `clear_map_releases_only_map_holds_and_preserves_files_settings_and_counters`; open: Q11 INI loader and unused-file selection |
| NetworkFileCache 6.3 presence/info/reads | `Cache::{present,present_ram,present_file,info,copy_info,size,created,data,file,force_file}` | `nfc01_through_nfc06`, `nfc07_through_nfc12`, `holders_loading_failures_paths_settings_and_persistence`; open: Q11 filename reservation |
| NetworkFileCache 6.4 admission/removal/maintenance/writer | `Cache::{add,add_sized,remove,maintain,write_ini}` | same three cache tests; open: Q11 source-file admission, INI escaping, native buffer allocator; writer edge choices Q15 |
| NetworkFileCache 6.5 NFC01–NFC12 vectors | cache test module | `nfc01_through_nfc06`, `nfc07_through_nfc12` |

## Vector index

The spec's constructed vectors execute in `engine/conformance/tests` as
`web_network_cookies.rs`, `web_network_cache.rs`, `web_network_http.rs`,
`web_network_http_lifetime.rs`, `web_network_www.rs`, and
`web_network_owner_settings.rs`, with deterministic foreign
replies, rather than as live HTTP/FTP requests. CK01–CK06, HTTP01–HTTP09 and
NFC01–NFC12 are labeled at their assertions. WWW vectors map as follows:

| Vectors | `web_network_www::tests` test |
|---|---|
| WWW-01–03 | `identity_default_hold_order_and_dispatch_target_selection` |
| WWW-04 | `handler_failure_owns_attempt_and_releases_after_callback` |
| WWW-05–06 | `local_threshold_open_fallback_and_read_failure_nonzero_id` |
| WWW-07–08,12 | `cache_threshold_and_failed_retrieval_continues_to_admission` |
| WWW-09–10,13–14 | `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry` |
| WWW-11,16 (FTP) | `constructed_components_and_ftp_admission_vectors` |
| WWW-15 | `progress_borrowed_no_waiter_update_and_cancel_all` |
| WWW-16 (news/HTTP),19 | `cancellation_late_resource_and_foreign_admission_zero_hold` |
| WWW-17–18 | `completion_conversion_failure_cookies_and_cache_before_target` |
| WWW-20 | `splitter_empty_slots_quote_and_directory_side_effects` |

## Closure disposition (2026-10-24)

Every settled contract above has an implementation and executable coverage. This
does **not** make any of the four native owners fully implemented: the published
spec itself excludes the following contracts. No transport, disk format or native
allocation policy is inferred to remove an open row.

| Owner | Fully implemented? | Sections that remain open |
|---|---|---|
| cGZWWWSystem | No | 4.1/4.13/4.14: parked Q7/Q9 residual checklist and request normalization; 4.4: Q9 cookie-header record construction; 4.7: Q10 null-target completion suffix |
| cRZHTTPClient | No | 5.1: Q13 date grammar/runtime adjustment, wire/framing/finalization/destructor/residual slots; 4.14: Q9 normalization. Directory/cache-setting assignments in 4.9 are implemented |
| cRZCookieManager | No | 3.2: Q1 port-list interpretation (literal n-1 implemented); 3.4: Q16 miss-side output (literal success-only assignment implemented); 3.6: Q3 binary/character conversion and unqualified constructor path; Q15 literal splitter/header/settings edge choices |
| NetworkFileCache | No | 6.1–6.4: Q11 constructor INI literal, filename reservation/collision, INI loader/escaping, unused-file selection, source-file admission and native allocator compatibility; Q15 writer order/read-side effects (literal direct byte-key-order materialization implemented) |

The module uses explicit foreign constructor inputs for unqualified path literals.
`Unsupported` is retained only at the open contract boundaries listed above (and
outside the native input domain); it never stands for an ordinary native failure.
The cache's `clear_map` drops only map RAM holds, preserves caller-held buffers and
does not delete files. The HTTP settings are logical fields without an invented
mapping to constructor string offsets. WWW applies them before the cache directory
result, retaining the documented side effects when that final operation fails.

No new game request is required for these unconnected core modules; no shadow key
or replacement key exists. Successful unit tests qualify the supplied spec contract
only. Q1/Q3/Q7/Q9/Q10/Q11/Q13/Q15/Q16 prevent a claim of complete native owner equivalence.
