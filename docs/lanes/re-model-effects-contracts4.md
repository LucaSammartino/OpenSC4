| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-model-effects-contracts4: GF19–GF24 native contracts | Codex / `re-model-effects-contracts4` (RE spec lane) | Bounded existing partial publication accepted by coordinator review; GF19–GF24 remain open; no native or whole-owner credit | Publish reviewed existing findings through the combined train; preserve parked worktree and original round count. No further RE round authorized. |

## Handoff
- STOP: lane guard PARKED by coordinator: prioritize verified/replaced; preserve findings, commit handoff, return blocked. No further RE rounds authorized; no merge queue file.
- Goal was GF19–GF24 only, native model-effects contracts and independent vectors; no engine reads/edits or capped adjacent-class work performed.
- Done: 7175a2df committed substantive §3.11.1–2 lifecycle/control/update-domain native proof before call 30.
- Done in stop commit: expanded graphics-formats §3.11.1–4 with complete 23-body inventory, shared destruction/getter, control widths, flags, creation/failure/toggle/recreation and caller source proof.
- Inventory measured by summing export body_bytes: 23 bodies / 8406 bytes; 16 owner-specific vtable bodies + ctor + clear + 5 map helpers. Slots 0–16; slot 3 shared getter 0060ce80 (4 bytes), outside total.
- Shared destruction 00513e00 (83 bytes) deletes nodes/strings/sentinel then releases instance; does NOT stop/release stored effects. Shared node destructor 00511900 (93 bytes) likewise has no effect release. Paired ctor/destructor callers cited in spec.
- Attachment changes call Shutdown before acquiring/installing new instance; same pointer no-op; attachment does not rebuild/start. Clear uses Stop(0); AddEffect replacement and RemoveEffect use Stop(1).
- GF21: native full 32-bit start/stop controls, not canonical bool; StartNamed third low-byte bool enables create-if-missing; named start forwards repeat calls, bulk start gates IsStarted; missing region still creates with null records.
- Native visual effect Start 005be920 forwards control (special value 2), sets started; Stop 005beba0 promotes 0 to 1 for effect bit04, otherwise zero requests deferred stopping without clearing started; nonzero clears started. CRT stricmp current thread locale verified at 00a04487.
- GF22: no owner visibility/camera/global/public-flags/index APIs; internal +0xc is count. Local transform flags 0 if no record, 5 if loaded; nonzero instance/local compose, outgoing flags instance OR5; zero shortcuts preserve instance or local respectively.
- GF23: no count/end check; address base + frame*28; null vector/computed-zero skips loading and preserves instance transform. Detached/empty update returns true. Animation QI A818EDD8 acquired/released temporarily, absent gives frame0.
- GF24: arg1 unused word (caller supplies world-position pointer), arg2 float[3] direction pointer, arg3 binary32 speed; only started entries get SetTransform then parameter15, count1, three float32 products. Caller 0050ebb0 passes automaton +40/+58/+94, with no conversion.
- Exact residuals: independent vectors were NOT written (first attempted vector patch denied by STOP); final spec/diff review and validation were NOT run; GF19–GF24 entries remain open and have NOT been moved/marked ANSWERED or indexed.
- GF24 physical speed unit is not established beyond raw caller field +94; no metres/second calibration claimed. Child runtime-specific start-control meanings beyond visual-effect forwarding are outside the owner proof.
- Next only if coordinator authorizes: review §3.11.1–4, add independent expected trace/numeric vectors, reconcile questions and run questions.py index; otherwise preserve this parked lane.
- No ABI changes/proposals, tests, game requests, shadow/replacement/default/owner-completion/coverage claims. No queue file written. Detached HEAD; supervisor owns publishing branch.
- REA batch failed target_unavailable (No app is open); used export instructions/data and decomp/_unnamed/region_005e0000.c; no new REA session opened. q.py Bash unavailable (E_ACCESSDENIED), direct PowerShell used.
- Ignored helper build-runs/native-data.py reads PE data; no game/decomp output committed. Manifest C:/Users/tlori/FreeCity4/build-runs/run4-re-model-effects-contracts4-r1-manifest.json.

## Coordinator review of bounded existing publication

Accepted the already committed partial contract/findings publication without changing the underlying spec or
closing GF19–GF24. The published inventory reconciles to **23 unique bodies / 8,406 bytes**; shared getter and
destruction bodies remain outside that total. No independent vectors or executed examples were added. Native
verification, exact arithmetic equivalence, runtime replacement and whole-owner completion receive no credit.

The phrase “checks ... and an empty key map” in §3.11.4 does not identify that map; it must not be used to invent
an empty-owner-map admission guard. Preserve the explicit discovery traversal and repeated-Init behavior, and
withhold any behavior that needs that ambiguity resolved. GF24's physical speed unit remains unknown; only the
published binary32 input and multiplication behavior is supplied. All original questions and residuals remain.

Read-only coordinator checks: the inventory count/sum above and `git diff --check` pass. This review does not
retroactively claim the worker completed its interrupted final validation. The old worktree's instruction-doc
baseline predates main's train100 fixes; the combined train must run doccheck with those fixes and its actual
required gate/regression. No ABI, engine, spec, coverage-map or default change is part of this review commit.
