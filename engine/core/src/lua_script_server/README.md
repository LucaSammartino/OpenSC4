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
