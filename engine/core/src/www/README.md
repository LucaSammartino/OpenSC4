# Portable web owners

Behavior source: `docs/specs/web-network.md` and its linked FTP/lifetime contracts.
The existing `rzurl` module remains the sole URL parser. No ABI/plugin integration,
OS filesystem, socket transport, or live game equivalence is claimed here. `Host`,
`Io` and `Runtime` supply foreign IO, clock, allocation identities and runtime
calendar conversion. Mutable operations require exclusive Rust access; getters
have no `RefCell`. Native references are explicit effects, except cache RAM buffers
whose portable ownership uses `Arc`; no-copy insertion transfers a `Vec` allocation.
Adapters must supply native allocator compatibility and stay within the specified
16-bit reference/32-bit size domain. Cookie/cache defaults use the settled
`CookieData.ini` and `FileCache.ini` literals; other path inputs remain explicit.

## Owner closure

Each row identifies executable coverage of the published contract. An `open:` suffix
is an existing spec limit, not a successful native false return. Rows do not claim
closure of the native address inventory. The q17/q18 spec answers are applied below.
Q19/Q20 record the newly identified initialization/record-transition gaps. Native
allocator/container wrappers are not claimed as portable owner coverage.

| Owner / spec section | Code item | Test / remaining boundary |
|---|---|---|
| CookieManager 3.1 record/defaults | `cookies::Cookie` | `parse_defaults_attributes_and_literal_port_rule` |
| CookieData 3.1 value record, defaults, copying | `Cookie::{default,clone}` | `parse_defaults_attributes_and_literal_port_rule`, `copying_lookup_does_not_alias_store_and_miss_preserves_output` |
| CookieManager 3.2 parser | `Cookie::parse`, `cookies::split`, shared `ftp::atol` | `parse_defaults_attributes_and_literal_port_rule`, `ck07_through_ck13_settled_edges` |
| CookieManager 3.3 admission/replacement | `Manager::add`, `add_text` | `admission_replacement_expiration_and_lookup` |
| CookieManager 3.4 lookup/header | `lookup`, `lookup_text`, `lookup_into`, `header`, `header_text` | `header_version_quirk_and_eviction_order`, `admission_replacement_expiration_and_lookup`, `copying_lookup_does_not_alias_store_and_miss_preserves_output`, `ck07_through_ck13_settled_edges` |
| CookieManager 3.5 eviction | `Manager::evict` | `header_version_quirk_and_eviction_order` |
| CookieManager 3.6 lifecycle/persistence/settings | `Manager::{default,new,init,shutdown,read,write,clear_file,enumerate,set_binary,set_file_path}`, `Host::{read_binary,write_binary}` | `ck01_through_ck06_and_lifecycle`, `persistence_read_cap_and_init_nonempty_guard`, `ck07_through_ck13_settled_edges` |
| CookieManager 3.7 inventory/vectors | portable methods above (native QI/refcounts are adapter roles) | `ck01_through_ck06_and_lifecycle`, `ck07_through_ck13_settled_edges`: CK01–CK13 |
| WWWSystem 4.1 identity/lifecycle | `service::Www::{construct_with_host,query_interface,post_init,shutdown,destroy}` | `identity_default_hold_order_and_dispatch_target_selection`, `constructed_components_and_ftp_admission_vectors`, `progress_borrowed_no_waiter_update_and_cancel_all` |
| WWWSystem 4.2 dispatch/holds | `Www::{load_text,load,set_default_target}` | `handler_failure_owns_attempt_and_releases_after_callback`, `identity_default_hold_order_and_dispatch_target_selection` |
| WWWSystem 4.3 local files | `Www::local` | `local_threshold_open_fallback_and_read_failure_nonzero_id` |
| WWWSystem 4.4 HTTP/cache/duplicates | `Www::admit_http`, `Client::submit_headers` | `cache_threshold_and_failed_retrieval_continues_to_admission`, `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry`, `http_admission_helper_spans_submission_and_www_insertion`, `www22_cookie_record_construction` |
| WWWSystem 4.5 status/cancel | `Www::{status,get_job,http_id,ftp_id,cancel,cancel_all}` | `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry`, `progress_borrowed_no_waiter_update_and_cancel_all` |
| WWWSystem 4.6 messages/progress | `Www::{envelope,progress,do_message}` | `progress_borrowed_no_waiter_update_and_cancel_all`, `duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry` |
| WWWSystem 4.7 completion/fanout | `Www::complete_http`, `convert` | `completion_conversion_failure_cookies_and_cache_before_target`, `cancellation_late_resource_and_foreign_admission_zero_hold`, `www21_null_target_skips_entire_suffix` |
| WWWSystem 4.8 FTP call boundary | `Www::{load,complete_ftp}`, `Scheduler::admit` | `constructed_components_and_ftp_admission_vectors`, `ftpc07_ftpc08_scheduler_admission`; fresh-session continuation open: Q19 |
| WWWSystem 4.9 settings/handlers/visited | `Www` settings/handler methods, `http::Client::{set_directory,set_cache_setting}`, `path_preference` | `splitter_empty_slots_quote_and_directory_side_effects`, `http_directory_and_cache_setting_are_independent_copies`, `handler_failure_owns_attempt_and_releases_after_callback`; implemented, including embedded HTTP state after failed cache directory creation |
| WWWSystem 4.10 WWW-01–WWW-20 vectors | service test module | named tests in preceding rows and vector index below |
| WWWSystem 4.11 path conversion | `www::{native_path,percent_decode}` | `published_native_path_vectors` |
| WWWSystem 4.12 response cache policy | `service::{cache_tokens,cache_policy}` | `policy_distinguishing_headers_and_wrapping_dates`, `splitter_empty_slots_quote_and_directory_side_effects` |
| WWWSystem 4.13 reached FTP bodies | `ftp`, `listing`, `worker`, `Session::{simple_command,acquire_listing_into}`, `Scheduler::{new,admit,destroy}` | FTP tests, `ftpc01_through_ftpc06`, `ftpc07_ftpc08_scheduler_admission`; fresh-session initialization open: Q19 |
| WWWSystem 4.14 reached HTTP admission | `http::{Client,Request,Job,method_id,method_name,parse_records}` | `method_prefix_and_wrapper_boundaries`, `admission_wrap_raw_cancel_leaks_and_first_match_wins`, `zero_and_one_request_ref_delete`, `norm01_through_norm08` |
| WWWSystem 4.15 identity/listener/socket/progress/name/format support | `Session::{query_interface,set_target,release}`, `ftp::{ensure_control,lookup}`, `Scheduler::{destroy,adjust_worker_reference,erase_range}`, `support` | `sup01_sup04_sup05_sup08_and_scheduler_destruction`, `sup02_sup03_sup06_sup07_sup09`; listing helper qualification open: Q19 |
| WWWSystem 4.15 value copies / native containers / adapters | `service::Job::clone`, `worker::Job::clone`, ordered Rust job collections; existing interface methods | SUP04/SUP05; native circular-sentinel allocations, deleting arrays and pointer-adjusting ABI wrappers are not modeled; owner not fully closed |
| HTTPClient 5.1 construction/lifetime/calendar | `Client::{new,complete,final_boundary,shutdown,worker_exit,query_interface,destroy}`, `http_date`, `notification::time_to_seconds` | lifetime tests, `http_slot01_slot02_and_http35`, `http26_through_http34_dates` |
| HTTPClient 5.2 synchronous notifications | `Client::send_notification`, completion effects | `http01_through_http09`, `cancellation_callback_before_release_and_stop_leaks` |
| HTTPClient 5.3 progress | `Client::update` | `http01_through_http09` |
| HTTPClient 5.4 HTTP01–HTTP09 vectors | notification test module | `http01_through_http09` |
| HTTPClient 5.5 / wire 1 request/header/transport | `http_wire::{request_bytes,expired,reusable,retry}`, `http_transport::{read_header,execute,Peer}` | `header_fragmentation_informational_status_zero_and_interrupts`, `worker_short_sends_numeric_lookup_reuse_and_framing`, `worker_failure_retry_redirect_and_error_codes`; native resource publication/proxy endpoint/status grammar open: Q20 |
| HTTPClient 5.5 / wire 2 body framing | `http_wire::{framing,Body}` | `http15_through_http21_body_framing`, `worker_short_sends_numeric_lookup_reuse_and_framing` |
| HTTPClient 5.5 / wire 3 finalization | `http_wire::finalize` | `http10_through_http14_and_http22_through_http25`, `worker_failure_retry_redirect_and_error_codes` |
| HTTPClient 5.5 / wire 4 dates | `http_date` | `http26_through_http34_dates` |
| HTTPClient 5.5 / wire 5 destruction | `Client::{query_interface,destroy}` | `http_slot01_slot02_and_http35` |
| HTTPClient 5.5 / wire 6 conformance | preceding code items | HTTP10–HTTP35 in `web_network_settled.rs`; transport fixtures in `web_network_transport.rs` |
| NetworkFileCache 6.1 defaults/settings/inventory | `cache::{Cache,Entry,Data}`, numeric setters | `holders_loading_failures_paths_settings_and_persistence`, `nfc01_through_nfc06`, `nfc13_through_nfc19` |
| NetworkFileCache 6.2 lifecycle/paths | `Cache::{init,shutdown,clear_map,read_ini,remove_unused_files,directory,ini_name,set_directory,set_ini_name}` | `nfc07_through_nfc12`, `holders_loading_failures_paths_settings_and_persistence`, `clear_map_releases_only_map_holds_and_preserves_files_settings_and_counters`, `nfc13_through_nfc19` |
| NetworkFileCache 6.3 presence/info/reads | `Cache::{present,present_ram,present_file,info,copy_info,size,created,data,file,force_file}` | `nfc01_through_nfc06`, `nfc07_through_nfc12`, `holders_loading_failures_paths_settings_and_persistence` |
| NetworkFileCache 6.4/6.4.1 admission/removal/maintenance/writer | `Cache::{add,add_sized,add_source_file,reserve_filename,remove,maintain,write_ini,read_ini}`, `cache::hash`, `Host::map_buckets` topology input | cache tests including `nfc13_through_nfc19`; writer visits supplied ascending buckets/chains, without lexical sorting |
| NetworkFileCache 6.5 NFC01–NFC19 vectors | cache test module | `nfc01_through_nfc06`, `nfc07_through_nfc12`, `nfc13_through_nfc19` |

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

The q17/q18 answers have executable coverage. This is a partial owner-closure
delivery: the table distinguishes published portable coverage from native wrappers
and the newly identified spec gaps. No whole-owner native equivalence is claimed.

| Owner | Fully implemented? | Sections that remain open |
|---|---|---|
| cGZWWWSystem | No | 4.13 fresh FTP initialization and 4.15 listing labels: Q19. Native 4.15 circular-list allocation/deleting-array/pointer-adjusting wrappers remain implementation work; this is not an unanswered question |
| HTTP (cRZHTTPClient) | No | Wire 1 transport is implemented; native dequeue/resource publication and proxy endpoint/status grammar qualification: Q20. Completion/lifetime algorithms remain separately exercised, not connected by an invented record cast |
| cRZCookieManager | Yes, published portable behavior | 3.1–3.7 covered, including CK01–CK13; IO/calendar and native allocation remain explicit foreign inputs |
| CookieData | Yes, published value behavior | 3.1 defaults/owned strings/pairs/fields and independent copying covered |
| NetworkFileCache | Yes, published portable storage behavior | 6.1–6.5 and NFC01–19 covered; filesystem primitives and native map bucket topology remain explicit host inputs; Arc ownership requires a native allocator/refcount adapter |

The module uses explicit foreign constructor inputs for unqualified path literals.
The FTP initialization `Unsupported` barrier now names Q19 instead of the incorrect
scheduler-constructor address. Cache Q11/Q15 barriers have been removed. Native
pointer identities passed without live borrowed storage remain outside safe inputs.
The cache's `clear_map` drops only map RAM holds, preserves caller-held buffers and
does not delete files. The HTTP settings are logical fields without an invented
mapping to constructor string offsets. WWW applies them before the cache directory
result, retaining the documented side effects when that final operation fails.

No new game request is required for these unconnected core modules; no shadow key
or replacement key exists. Successful unit tests qualify the supplied spec contract
only. Remaining engine integration prevents a claim of complete native owner equivalence.
