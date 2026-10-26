| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-mysims27 (engine, MySim replacement safety) | Codex / `replace-mysims27`, detached HEAD; supervisor publishes reserved branch | ready; code `710a7aa10`, `75fafcb84`, `f102e437a`; ABI +5 exports; seam none; Rust 7849/0, clippy 0, native 51/51, CTest 118/118; save-7/reload-2/house-1/shadow-2 clean, refs 0; shadow 73/73 exact; questions none | 1. Publish detached tip and land the ready queue entry. 2. Add mysims to the replace list. |

- mysims is ready for the replace list. All three game proofs are measured; every successful acceptance run has ref_warnings=0.
- Final source `f102e437a`: qbuild all PASS, Rust 7849/0, clippy 0, native 51/51, CTest 118/118. Logs: WT `build-runs/{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log`.
- The first round-3 all failed only ret_channel_check; the missing GetPosition annotation was fixed and focused/final checks passed. Earlier failed gates retain no acceptance credit.
- (1) 69-slot cISC4MySim front, advisor identity/refcount, both factories and building/occupant conversion: `plugin_mysims_replace_factory_test` PASS in final CTest log.
- (2) Advisor Write/Read use codecs, owning/restoring dispatch references. `advisors_tests::mysim_record_round_trip` PASS, including the serialized personal name, in final Rust log.
- (3) Constructed advisor core is published before Init; first QI takes the factory reference. Five-object `advisors_tests::mysim_factory_before_system_init` and native factory test PASS.
- (4) Lua tables/context/fields and tuning_constants reader are connected under mysims sections 4.4/4.11; no separate live tuning-value assertion.
- Round-3 fixes: use AsOccupant/QI for position, lot, utilities and dispatch; use the lot manager's B allocator; request all occupants with maxCount=-1; read the specified uint8 age grid. Script move-in scans city lots.
- Acceptance logs below are in `C:/Users/tlori/FreeCity4/build-runs/game-results/`; every request used top-level parallel=1.
- Save `replace-mysims27-r3-save-7.log`: clean; active=1 moved=1; FC4 Persist, slot=3 state=1 age=40 wealth=1 generation=1 home=16,94 bio=replace-mysims27 portrait=0; SaveCity(full) ok=1; export hash 6f44b7f031fb0dada79dc30e856ed9c51903e6ee78ac347315f208a26f6b5864.
- Reload `replace-mysims27-r3-reload-2.log`: prepared_city stages that exact hash; clean through 2026-03-07; both queries active=1 and all listed simulation/portrait/bio fields restored. No Documents access by the lane.
- Reloaded display name is blank after Init: advisors.md section 4.4 explicitly rereads the personal name from the Lua advisor entry; the codec itself retains FC4 Persist. This difference is explained by the spec.
- House `replace-mysims27-r3-house-1.log`: exact operator-supplied default list plus mysims; clean 2032-01-01, 276.421 s, 7.83 days/s; foreign/watchdog/RTC/overrun/ref counters all 0; switches_off=-, disabled=-.
- Shadow `replace-mysims27-r3-shadow-2.log`: mysims_shadow=1 with native MySim/advisor classes and enable=clock24,effects; clean 2032-01-11; stopped months=73 clean=73 last_mismatches=0 (73/73 dated comparisons exact).
- Failed attempts are separate: old r2-save-2 export path rejected; r3-save-1 slot occupied, save-3 duplicate export, reload-1 fixture path rejected; save-2/4 had no resident; save-5/6 exposed the now-fixed pointer/allocator faults. No credit from these labels.
- Shadow-1 failed in native cSC4EffectsManager::DoZoomChanged during load; shadow-2's effects replacement avoids that native path while leaving the compared MySim classes native.
- MySim getters use RwLock. UI controls/audio/camera/balloons remain delegated per mysims section 0; move_in_control remains unwired; scripted move-in proves persistence through the public interface.
- Questions: none. No outstanding game requests or exec sessions. Shared refs/config were not changed. Ready queue metadata uses the full final detached HEAD; supervisor publishes the reserved branch and attaches train coverage credit.
