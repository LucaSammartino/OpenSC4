| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-prop-occupant4: first independent plain prop scalar native verifier | Codex, verify-prop-occupant4 | READY: bounded native exact; gate and OFF/ON clean | Queue exact tip; no automatic subsequent round or coverage/default promotion |

## Implementation and frozen scope

Added default-off `prop_occupant_verify=1` / `FC4_PROP_OCCUPANT_VERIFY` at city load, using supported GZCOM GetClassObject(2977AA47,E9793A65) and two independent existing Rust/C ABI handles. No owner implementation, C export, ABI version, generated entry/registration, maps, defaults or factory policy changed; cores21 history preserved. Detached HEAD retained.

Inspection preceded implementation and any gate: existing create/destroy, orientation i32, state u32/set byte, chance/usage bytes, timer full-EAX getters/byte setter and float time outputs were sufficient. Host primitives are independent constants (not-in-city, no exemplar); unexpected message/model/clock/RNG/removal callbacks count as forbidden. Native values are outputs only, never Rust inputs or expected values.

Oracle and supported table frozen in e2f07144 before comparator88b6b272. Literal45-mutation transcript: object0 orientation MIN/-1/0/1/2/3/4/MAX/-128; chance0/1/99/100/255; usage0/1/2/3/4/255; repeated state0/1/1/2/255/128/0; timer80/C0/C3/C7/C5/CD/4D/0D/0C/00/F0/00/00. Object1 then orientation2/chance1/state255/usage3/timer80. Both objects' entire adjacent scalar states checked after each mutation; masks0/1/2/8/80/FF. Low timer nibble never equals F. Fresh defaults independently literal0/0/100/0/0 plus range+0/+0.

Local published24-slot declaration corrects the SDK's narrowed state12/timer19/20 to clean EAX. Orientation setter10, chance18 and usage22 use byte AL normalized to8bits without bool conflation. Void setters have no invented result. Range14 compares both output float bits. No Init, city insertion, attachment, shared clock/date/RNG mutation, type table or private field access.

Before every direct native dispatch: existing exact1.1.641 gate, published primary table00AA4900, and executable dispatch-target ownership in main module00400000. Only interface-pointer/module identity is inspected. Actual configuration and factory identities/reasons logged; refusal stops rather than searches alternatives. Replacement cleanup receives no native credit.

## Actual bounded result

ON completed1; factory attempts/success2/2; attempted/dispatched1065/1065; refused0; mismatches0; snapshots92; mutations45; forbidden0; natural0. The full actual method denominator includes lifecycle calls. Every scalar output is checked independently against its literal oracle on native and Rust; native/Rust equality follows, not a third counted assertion.

`compared_values=1032`:1026 scalar/out/AL-return observations, two QI identity/static-CABI-interface comparisons, four native-only Release/count assertions. `literal_checks=2064` is the two-operand instrumented count: four Release companion operands are fixed literals, not Rust COM outputs. Meaningful observed-output assertions=2060. Rust handle create/destroy is counted separately; no Rust/native COM-refcount-parity claim.

All rows: attempted=dispatched; refused=mismatches=0. Void setters' postconditions are in the following two full snapshots, so their compared-return count is0.

| Slot / method | Supported / partial / unsupported scope | Calls | Compared values |
|---|---|---|---|
| 0 QueryInterface | partial: primary IID identity/acquired ref | 2 | 2 |
| 1 AddRef | unsupported direct; QI implicit only | 0 | 0 |
| 2 Release | partial: owned refs/native returned counts | 4 | 4 |
| 3 AsOccupant | unsupported | 0 | 0 |
| 4 SetBoundingBox | unsupported | 0 | 0 |
| 5 GetPropPlacementProperties | unsupported | 0 | 0 |
| 6 SetPropPlacementProperties | unsupported | 0 | 0 |
| 7 GetPropType | unsupported global intern state | 0 | 0 |
| 8 SetPropType | unsupported global intern/ambiguous return | 0 | 0 |
| 9 GetOrientation | supported isolated scalar | 92 | 92 |
| 10 SetOrientation | supported signed clamp counterexamples/AL1 | 10 | 10 |
| 11 SetState | partial: unattached repeated byte changes, void | 8 | 0 |
| 12 GetState | supported byte state/clean EAX | 92 | 92 |
| 13 SetTimeOfDayRange | unsupported shared clock mutation | 0 | 0 |
| 14 GetTimeOfDayRange | partial: constructor+0/+0 only | 2 | 4 outs |
| 15 SetSimulatorDateRange | unsupported | 0 | 0 |
| 16 TickSimulatorDate | unsupported | 0 | 0 |
| 17 SetRandomChance | supported isolated byte store, void | 6 | 0 |
| 18 GetRandomChance | supported byte AL, not bool | 92 | 92 |
| 19 GetPropTimerBitfield | supported byte storage/clean EAX | 92 | 92 |
| 20 GetPropTimerState | supported EAX0/1, six masks | 552 | 552 |
| 21 SetPropTimerState | partial: high bits/incomplete low masks, void | 14 | 0 |
| 22 GetPropUsage | supported byte AL including255 | 92 | 92 |
| 23 SetPropUsage | supported isolated byte store, void | 7 | 0 |
| Total | bounded constructed fixture | 1065 | 1032 |

Distinct native pointers1A959188/1A9591F0: table00AA4900, QI target005EC960/module00400000, original1, replacement=none. Each QI ref released to1, factory ref to0. Factory2/2, QI2/2, Rust2/2, foreign cleanup0, lifecycle balanced1. Object0 host queries city5/exemplar5; object1 city1/exemplar1; forbidden0. Per-object and per-method authority identities retained in ON log; every dispatch rechecks authority.

No natural prop/developer, creation-selection, placement/AABB/lot-tag, global intern, clock/date/tick, model/effects, manager, record/read/save-owner, draw, whole-owner or coverage/default-promotion credit. Save events measure harness continuity only. No reload claim. CURRENT40 unused: its raw literal40 and injected actual42 profiles were not run. This native-preserving profile is optional-literal0/actual2, explicit budget,simulator; propmgr disabled, all other optional providers default-off.

## Gate, tests and artifacts

One final canonical qbuild all after source88b6b272: PASS exit0; cargo5262 passed/0 failed/0 ignored across22 result lines; clippy0 diagnostics; native216/216; full CTest68/68 (121.3s). fc4verify1027.6s overall/build84.4s. Session24738 collected final exit0. No baseline, quick, overlapping or duplicate all. No Rust touched (rustfmt n/a); C++ local formatting and diff --check clean.

Wrapper noted `[the tree changed during the run]`: only committed lane/run documentation changed. `git diff 88b6b272 HEAD -- engine` empty. Sorted tracked engine byte-hash before/after all identical, excluding generated abi_entries.inc: **19a922a3ecb0f4238e3d0f0d4261be988884bc8b57a78a2305ff65848a1474fd**. Engine Git tree **d26f2e9faafce7a7e837d2c1d199188bf8c9dba1**. Hash definition/helper build-runs/source-hash.py. No unchanged rerun was needed to establish stable source.

Existing plugin-load CTest executes isolated comparator controls: exact45-mutation/1057 scalar-dispatch transcript; seven injected faults (clamp, full-EAX contamination, chance bool, timer width, usage bool, cross-field contamination, clear mask); object aliasing; AL upper-bit acceptance/full-EAX rejection; refusal stops both endpoints before dispatch; production owned-release boundary with exact/denied/over-release callbacks. Only local doubles are faulted; no owner expected-value changes.

Frozen once in build-runs/frozen-prop-occupant4 with pair.json, rehashed unchanged after both runs:

- DLL SHA256 **76f73c7dc69e1cfe8783782ad33fd09e01b77cad4a7d925d787f148224c285e8**.
- PDB SHA256 **c57c485a663319df6bb00887d6975e2078502df1c76a9deaf8330f57571749bc**.
- Shared script SHA256 **89088da00ef30dd987a1d04feddc16faed6a5bf8a484b9c93afdb4779d9c9eda**.
- Clippy log17a9ad466c1285a99320c6f982d914947e01db2f66a7b0d00a484aaf25ed1af2; build log809f58eecb47978c9b6cc9c617d0c209408712163bc023291ab30898e2ee104f; CTest log4dc0fb199bbe6529a10fd4e8f2a61871bae0bda5d18b8e95b1ef4cf7c03085e9.

## Sequential game controls

Same frozen DLL/PDB, providers, Konradshohe, turbo1/speed2, seconds900/timeout1100/end2032-01-01, refaudit1 and quit. Same three-action script: mark, full save2031-12-01, mark2032. Both clean at2032-01-04, foreign exceptions/stalls/RTC/overruns/ref warnings0; all3 actions and PostSystemServiceShutdown completed; simulator shutdown held0 refs each.

- [OFF vpo4-r1-off-2032-20261023a](../runs/vpo4-r1-off-2032-20261023a.md): wall56s/real43.187s, save full11031ms on2031-12-16, no verifier lines/no native-exactness credit.
- [ON vpo4-r1-on-2032-20261023a](../runs/vpo4-r1-on-2032-20261023a.md): wall138s/real121.594s, save full12454ms on2031-12-04, exact bounded counts above, DLL detach logged.

Both actual requests preserved in build-runs/game-queue/done; unique verdicts/logs in build-runs/game-results. Run records hold full evidence hashes. Waited using local prop-evidence.py, no qwait; completed unique verdicts accepted. No failed requests/retries, extra CURRENT40 run or additional pair. Both waiter sessions collected exit0; no outstanding request/gate.

Initial q.py Bash launch failed E_ACCESSDENIED; subsequent reads used bounded PowerShell. One Windows process diagnostic denied; no escalation, cache RPC probes, kills or policy edits. Git packed-refs.lock diagnostics did not prevent detached-HEAD commits; no manual refs/config/hooks changes. No private/RE/tool-source/other-worktree reads or external messages/subagents.

## Questions and next steps

None filed; no differences; existing residuals untouched. ABI exports unchanged. Queue this exact clean tip for measured bounded success only. No subsequent implementation round, owner change, coverage/default/generated-table edit or natural/whole-owner qualification is authorized by these results.

## Coordinator provenance and numeric-record correction

At02:33:56.905879 BST root terminated ONLY exact stalled read-only stats client13832, creation1792718419.496703,
after independent review and fresh PID/argv/cwd/parentage/service/output/source proof. Exit15; SAME validator7824
and daemon19696 survived, proceeding to CTest15608. This occurred after native control work began; actual final
68/68 CTest completion is later evidence tied to the same engine source, not a gate finished before OFF.
Cache statistics unavailable/interrupted; RPC causeUNKNOWN. No daemon/compiler/source/check/cache-policy
restart or duplicate validation. Earlier strict preflights stopped before action on report-only HEAD movement
and an unfamiliar read-only verdict collector; unchanged-engine/collector proof resolved both. Exact own JSON
and textual intervention notes retain the evidence. The worker's lack of process actions does not imply no
coordinator action.

Coordinator assigned OFF/ON record IDs289/290 after an exact collision-free scan across123 run directories.
Original OFF285 duplicated accepted train108/run285; original ON286 is retired numeric metadata, not another
execution. Unique labels and all raw outcomes remain unchanged. Command controls receive287/288; train109
postland291 is reserved pending actual acceptance. No source, expected value, test, native call, game run,
measured count or original1/3 budget changed. Reported2064 literal checks include four fixed release-companion
operands;2060 meaningful literal operands retain their bounded domain, not whole-owner qualification.
