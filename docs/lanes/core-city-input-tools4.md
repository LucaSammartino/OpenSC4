| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-city-input-tools4: portable Flora, Torch, CitySituations | Codex; `core-city-input-tools4` (reserved detached worktree) | WIP: Rust 5348/0 and Clippy pass; native174/174, exact native exit and CTest unverified after cache-stats stall | Hold; coordinator must resolve infrastructure and obtain complete native/CTest68 results before READY. No automatic extra round. |

## Implementation and provenance

Exactly three new portable production owners in `engine/core/src/city_input_tools/`.
The base is train108 `640ba764`, including the required PlayerDrive landing.
`d6ad7d1e` committed the named supported/partial/unsupported inventory and
independent literal fixture traces **before** production source. `9ac71616`
committed the implementation, tests and questions. The final canonical gate
tested this committed, clean source; no source edits occurred during or after it.
Only the shared `lib.rs` module declaration was appended; existing owners stayed
unchanged. No C ABI, ABI version, plugin, native adapter, game request, UI,
clipboard, native coverage/maps/defaults or generated ABI file edits.

Flora computes its hit/funds branches, pressed/hover/shortage state, checked
int64 stroke total, press-versus-drag withdrawal asymmetry, sound selection and
position, indicator intents and aura calls. Insertion's rewritten position is
used. Torch computes named effect transitions and release ordering, raw modifier
0x10 activation, transforms, immediate press versus retained active-move cells,
ignition flags/cause order, tick registration/removal, cursors and audio intents.
CitySituations computes lock/pressed handles, hover transitions, level gates,
indicator states, matching-press mission decisions and all three reset messages.

Hosts supply primitive picks, raw world coordinates, funds/insertion/withdrawal/
location/water results, opaque resources, effect-started state, timer counts,
indicators, occupant and service presence. They never supply final tool handler,
stroke, cell-selection, effect-transition or mission decisions. Existing
Vec3/Pixel and ControlHost cursor operations are reused. Base result sound is
the slot11 primitive, not the existing different event-post helper. Held flora/
fire and created effect obligations are released explicitly; borrowed timer,
indicator, view and occupant identities acquire no invented COM obligation.

## Exact per-method report

Supported below means the published **portable** algorithm in its stated domain.
No complete native owner or binary inventory is claimed. Published body byte
lengths/end addresses remain unknown; no byte totals or whole-owner credit.
The pre-source full inventory and oracles are in the module README and fixtures.

| Owner / method | Result and remaining boundary |
|---|---|
| Flora Init 004BBA20 -> init | Partial: constants, supplied static price/acquired simulator, zero total and price indicator; base Init/lookup/native return unsupported |
| Flora Shutdown 004BB940 -> shutdown | Partial: acquired simulator release then clear(true); native base/return unclaimed |
| Flora planting helper -> plant | Portable algorithm: insertion(false), positioned instance sound, withdrawal, conditional press/unconditional drag total, indicator, aura; failed-insertion water/result branch; CT15 callsite details literal |
| Flora Press 004BBAF0 -> press | Hit/funds gate; pressed even unaffordable; true in supported domain |
| Flora Move 004BBC80 -> mouse_move | Pressed planting/shortage or hover predicate, sticky shortage sound on every move, true |
| Flora Release 004BB980 -> release | Partial: total-dependent indicator, reset pressed/total/shortage, true; native release-clear flag unresolved |
| Torch Init 00580660 -> init | Partial: constants/inactive/fresh cell0, borrowed timer start, on intent; native acquisition/base/return unsupported |
| Torch audio helper 00580550 -> TorchAudio intent | Partial: exact published message selection/data; CT2 event creation/post/consumer unsupported, unknown payload explicit |
| Torch ActivateTorch 005806D0 -> activate | Effect/change gate, stop0/release/create/start0, deactivation-release and active intent; foreign failure typed residual |
| Torch UpdateTorchEffect 00580760 -> update_effect | Retained position/identity rotation/scale1; missing position typed residual |
| Torch ShutdownTorchEffect 005807F0 -> drop_effect | Started check, stop1 only if started, release, inactive/active(false) intent |
| Torch OnMouseExit -> mouse_exit | Partial: effect drop; no invented native handler return |
| Torch Move 00580A30 -> mouse_move | Pick/miss drop, lazy inactive effect, raw0x10 activation, placement, last-active-move cell, move intent, true |
| Torch Press 005808B0 -> press | Press intent before pick; activate existing effect only, place, registerC0, immediate hit ignite(false,true,0), cause1, cursor; retained cell unchanged |
| Torch animation 00580510 / 86AD10EE -> animation_tick | Active gate, retained move cell ignite(true,true,0), cause1; no guessed message return |
| Torch Release 005809D0 -> release | Release intent, unregister, deactivate/replacement, own cursor, true; duplicate deactivation release literal (CT17) |
| Torch Shutdown 00580840 -> shutdown | Partial: timer stop, effect drop, off intent, held fire release, unregister; native return/timer ownership unclaimed |
| CitySituations construct 004B8690 -> new | Partial: identity constants; constructor handle values explicitly supplied, CT14 |
| CitySituations Init 004B84B0 -> init | Partial: subscribe A6B79602 only; base/manager acquisition and lifetime unsupported |
| CitySituations Move 004B8620 -> mouse_move | Partial: lock/state3, same-handle retain, different/missing hit unlock/forget without relock; empty-hover residual, CT16 |
| CitySituations Press 004B8720 -> press | Level1 gate, hit state2/remember/true; otherwise false |
| CitySituations Release 004B8790 -> release | Partial: level1/managers gate, state1 before matched+occupant initiate(null advice); literal hit returns true/retains pressed, failure clears/false; CT16 |
| CitySituations DoMessage 004B85B0 -> message | Three IDs unlock held handle, state(0,1), forget both, true; unknown result residual, CT16 |
| CitySituations Shutdown/native lifetime | Unsupported: no published body contract; no fabricated cleanup or return |
| All native COM/layout/full base Init/stack and unspecified constructors/failures/returns | Unsupported; no raw pointer or native replacement |
| Portable state getters | Shared references/value copies, no RefCell; no additional native method attribution |

Unsupported domain is explicit: nonpositive price, overflow, nonfinite/out-of-range
conversion, foreign resource failure, invalid lifetime, missing position,
empty-hover and unknown message. Completed primitive prefixes are preserved on
error; no clamping, wrapping, fallback resources, null defaults or timer defaults.
Int64 cost intents have no invented int32 wire conversion; release-clear intent
does not fabricate a bool. CT1 key/modifier/settings and CT2 audio uncertainty
remain intact.

## Measured validation

- Exactly one focused `qbuild quick`: **exit1**. E0277 from deriving Eq for a
  new audio value containing existing Pixel (PartialEq only), zero Rust result
  lines; dependents same compiler failure; native43/216 edges, one failed edge.
  Fixed only the new derives before commit9ac71616. Original logs archived in
  `build-runs/evidence-city-input-tools4-quick/`; no repeated quick.
- Exactly one final stable-source canonical `qbuild all`: **exit1 / FAIL**,
  fully collected. `qbuild-test.log`: **5348 passed, 0 failed, 0 ignored** summed
  over **23** result lines. `qbuild-clippy.log`: **exit0, 0 diagnostics**.
- Final native log: release Rust compile finished in68s, Ninja **174/174**,
  DLL/plugin-load-test linked, no failed edge/error recorded. Exact native exit
  and fc4verify parsed verdict are **unknown**, not inferred from the last edge.
- **CTest68: unverified / not run**, no CTest result log. The verifier stalled
  after native build in its own `sccache --show-stats --stats-format json` child.
  No native test count or native/shadow pass is claimed. Game/shadow: n/a;
  explicitly no game/adapter work in this task.
- Absolute-file `qbuild fmt` passed all **five owned Rust files**, with inherited
  warning `could not canonicalize path C:\Users\tlori`. Shared lib edit is one
  appended declaration; no mass reformat. No unchanged baseline all was run,
  following the task-specific instruction.
- Before/after **six touched source SHA256s match exactly**, clean tracked tree
  at the gate's source commit9ac716168efbc2b4dcf582f1ab04b639f44b1d85. Source
  manifests: `build-runs/city-input-tools4-before.json` and `...-after.json`.
  Baseline lib blob SHA256 (d6ad7d1e) is
  `ecaac3d6f0b6aff32e19960831066beeb996933299f54c69108af785a78b1ffa`;
  new source module was absent then. Final source SHA256s:

| File | SHA256 |
|---|---|
| lib.rs | b919b9c8c827d8a4bf654c8ffb06ad2a5bc4b77faa5f4b96cd3b3531f91e8377 |
| city_input_tools/mod.rs | 861e0fbbf7a801dde3e66d77d9d669dbe6973531e8f358feb42ae6ce05285f4e |
| city_input_tools/flora.rs | 5f992db2b446aeb9ef593eda83cd14ee111e1ee8c09e466d2969b6a76c624735 |
| city_input_tools/torch.rs | 5ee16333a17e07d277bf3293ca654f84f269b0ab5b96e0382326b00f2b43f757 |
| city_input_tools/situations.rs | 37531a44257ddd33823f43c15e692c9b7e22c98628773ad3ae8229ae6e97db43 |
| city_input_tools/tests.rs | 86f51d4fc8cbf0fca2a6c3eaa4b9b2c14215c4ee03485c2b3c97181b0496fe1c |

All 19 new individual results from final qbuild-test.log are **ok**:

| New test | Result |
|---|---|
| coordinates_outside_portable_domain_are_residual_not_saturated | ok |
| flora_failed_plant_water_vertex_truncates_toward_zero | ok |
| flora_failed_withdrawal_press_and_drag_are_counterexamples | ok |
| flora_hover_miss_and_shutdown_do_not_invent_state_or_releases | ok |
| flora_int64_total_and_overflow_preserve_truthful_prefix | ok |
| flora_poor_press_latches_and_zero_release_resets | ok |
| flora_shortage_repeats_on_every_move_even_miss_and_recovery | ok |
| flora_success_uses_insertion_rewritten_position_and_press_total | ok |
| situations_all_three_reset_messages_have_identical_order_despite_one_subscription | ok |
| situations_hover_transition_unlocks_without_immediate_relock | ok |
| situations_level_gate_refuses_and_release_clears_pressed | ok |
| situations_missing_indicator_or_manager_refuses_and_forgets_press | ok |
| situations_release_sets_state_before_matched_mission_and_ignores_manager_result | ok |
| torch_foreign_creation_failure_preserves_prefix_without_fallback | ok |
| torch_miss_exit_and_shutdown_order_distinguish_stop_flags_and_releases | ok |
| torch_press_hit_and_first_tick_use_different_cells | ok |
| torch_press_without_effect_or_hit_does_not_manufacture_activation | ok |
| torch_raw_modifier_bit_and_same_state_gate_are_literal | ok |
| torch_release_replacement_and_next_tick_retain_last_active_move | ok |

## Blocker, process provenance and operational errors

Final command: `python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> all`, interpreter
`C:/Program Files/Python312/python.exe`. Own qbuild PID22964 -> verifier PID29764
(`fc4verify.py --wt <WT> build ctest`) -> cache-statistics client PID29816
(`sccache.EXE --show-stats --stats-format json`). Native log completed at
2026-10-23T02:40:56+01:00. Statistics client remained stalled for over11 minutes.
At02:52:29.983849+01:00, after exact command/parent verification, **only own
Python verifier PID29764** was cancelled with exit1 to bound the final check.
qbuild then exited1 and its async session55863 was fully collected. No own
qbuild/verifier gate is outstanding. Cache client PID29816 was left untouched;
its eventual exit/liveness is unknown. No server/client kill, cache RPC probe,
policy/environment change or wrapper-source change was made. Evidence and
read-only process metadata helper are retained under own build-runs; cancellation
helper records its exact target. No second final gate was launched.

Other retained errors: q.py failed Bash/Service/CreateInstance/E_ACCESSDENIED
before its shell command; bounded PowerShell reads used thereafter. Initial fmt
relative paths failed (wrapper cwd differs); an attempted qbuild-fmt.log read
found no such file, and first absolute fmt reported formatting diffs, fixed only
in new files with apply_patch. CIM process metadata access denied; psutil absent;
Windows metadata queried directly for own process attribution, without cache RPC.
Two diagnostic PowerShell formatting pipelines emitted OutOfSequencePacket and
exit1; no verdict inferred from these. Git commits printed packed-refs.lock
permission errors but exited0 and created their detached commits normally; no
shared refs/config/hooks changed. Exploratory rg no-match exits are not gates.

## Questions and next steps

**CT14-CT17** appended to `docs/specs/questions/city-tools.md` (continued shared
CT prefix after CT13): supplied initial/invalid primitive state and failure
domain; indicator wire widths/clear flag/planting callsite details; ambiguous
CitySituations empty-hover/release/reset behavior; Torch payload/timer/failure/
deactivation details. Generated QUESTIONS.md was not edited, and prohibited
tools scripts were not run. Coordinator may regenerate the index separately.

Hold this clean committed lane as **WIP**, never READY on the native edge count.
Coordinator must resolve the infrastructure block, preserve the unknown cache
client exit, and obtain exact native exit plus the full required CTest68 gate
before landing. Clarify CT14-CT17 before extending the stated portable domain.
Any further implementation/round requires separate review; no automatic restart,
extra round, native adapter or other-owner continuation is requested or claimed.
