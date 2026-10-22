# ScriptServer owner inventory

Behavior source: `docs/specs/lua-host.md` section 6.3 and the public
`docs/lanes/re-lua-scriptserver4.md` report. No analysis artifacts were consulted.
The public inventory's 50 real attributed bodies / 7658 bytes map as follows.
Addresses identify entries in the public specification; no native binding is added.

| Public entry | Rust responsibility |
|---|---|
| 008B2540 | arguments.rs: setup_thread_arguments/table_value |
| 008B32B0 | mod.rs: run_script_string |
| 008B2380 | mod.rs: on_tick/tick_pass |
| 008B13A0 | mod.rs: enumerate_script_ids |
| 008B0860 | mod.rs: init |
| 008B42C0 | breakpoints.rs: find_or_create_breakpoint/find_breakpoint |
| 008B1180 | mod.rs: pause_script/control |
| 008B1290 | mod.rs: resume_script/control |
| 008B1A80 | mod.rs: run_script_file |
| 008B1E00 | breakpoints.rs: should_break |
| 008B22B0 | mod.rs: kill_script |
| 008AED10 | arguments.rs: setup_global_arguments |
| 008B51A0 | breakpoints.rs: set_breakpoint_condition |
| 008B4210 | breakpoints.rs: get_breakpoint_entry_list |
| 008AEBC0 | mod.rs: notify with script payload |
| 008B4EC0 | breakpoints.rs: set_breakpoint_exists |
| 008B4F60 | breakpoints.rs: toggle_breakpoint_exists |
| 008B5070 | breakpoints.rs: set_breakpoint_enabled |
| 008B01C0 | mod.rs: shutdown |
| 008B0410 | mod.rs: get_script_info/InfoOutputs |
| 008AEC60 | mod.rs: notify with no payload setter |
| 008B5110 | breakpoints.rs: get_breakpoint_condition |
| 008B07C0 | mod.rs: new/Default |
| 008B5000 | breakpoints.rs: get_breakpoint_enabled |
| 008B1B80 | breakpoints.rs: first-match removal in set/toggle existence |
| 008B4E60 | breakpoints.rs: get_breakpoint_exists |
| 008AE850 | arguments.rs: line_hook |
| 008B04E0 | mod.rs: set_script_auto_yield_enabled |
| 008B04A0 | mod.rs: get_script_auto_yield_enabled |
| 008AE7D0 | mod.rs: query_interface/InterfaceView |
| 008AECF0 | mod.rs: get_current_running_script_id |
| 008AE810 | mod.rs: release |
| 008AE830 | mod.rs: release dispatches shutdown then deleting_destructor |
| 008B10C0 | mod.rs: init, same owner through service view |
| 008B0400 | mod.rs: shutdown, same owner through service view |
| 008B3200 | mod.rs: Record::new |
| 008B1540 | breakpoints.rs: Vec::remove/drop of first matched payload |
| 008B0320 | mod.rs: deleting_destructor's plain destruction, no implicit Shutdown |
| 008B0520 | mod.rs: Breakpoint::default |
| 008AE8B0 | arguments.rs: argument_index |
| 008B2250 | mod.rs: owned Record payload destruction/erase_record and rejected creation |
| 008B1D00 | mod.rs/breakpoints.rs: owned Vec payload/sentinel storage destruction |
| 008AFCF0 | mod.rs: tick_pass live signed-order successor traversal |
| 008AFD40 | mod.rs: BTreeMap node storage destruction |
| 008B0840 | mod.rs: deleting_destructor, flag bit zero controls allocation free |
| 008B03A0 | mod.rs: query_interface, same owner through service view |
| 008B03B0 | mod.rs: release, same owner through service view |
| 008B03C0 | mod.rs: query_interface, same owner through destruction view |
| 008B03E0 | mod.rs: release, same owner through destruction view |
| 008B03F0 | mod.rs: deleting_destructor, same owner through destruction view |

Additional named/folded dependencies, outside that 50-body count: AddRef (008AE800
and view adjustment 008B03D0), GetLua (00A6CAB0), last ID (006AAC50), class ID
(008AE770), factory (008B10D0) and signed map-find (006C0030) are represented by
add_ref, get_lua, get_last_created_script_id, CLASS_ID, factory/new and BTreeMap lookup.
The four shared list allocation/copy/clear helpers are Rust owned Vec/Clone/drop.
Native pointer adjustments and sentinel layouts become shared Rust owner identity
and standard containers, rather than new ABI views.

00677261 / 42 bytes is the public report's false string-argument reach attribution.
It has no owner body and no invented callback. Attribution correction is external
to this lane; no coverage maps are edited.

## Boundaries and validation

`Foreign::invoke` accepts typed VM/thread, stack, command-generator, parser,
string-folding, file, base-service and message primitives. The Rust owner produces
the scheduling, control, table writes and notification protocol. No ScriptServer
result is delegated. Callback reentry is immediate; nested ticks alone are
suppressed. Invalidation of a borrowed record or active lifecycle is a typed
`InvalidatingCallback` failure with no rollback/deferred-control guarantee.
Malformed replies, duplicate IDs, null successful threads and unavailable required
message storage also have typed failures. Successful paths require available Rust
storage and valid foreign interfaces. The embedding layer must explicitly call
Release/destruction with a live foreign provider; Rust Drop cannot call an absent
provider. There is no native binding, runtime shadow or replacement claim.

`engine/conformance/tests/lua_script_server.rs` contains exactly 31 independent
public vectors in the three section 6.3 tables' order. Additional edge tests live
in `lua_script_server_edges.rs` and the core's `tests.rs`: lifecycle, ownership,
callback order, failure/retry, counter widths, stack protocol and live traversal.
