| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrec-lr14: construction lifetime and method inventory | Codex / `re-lotrec-lr14` (RE; detached HEAD) | ready: LR14 answered; six vectors T74–T79; questions/doccheck pass; ABI unchanged; no new game runs | Coordinator: use passive lifetime trace before claiming turbo construction closure |

## Report

Answered **LR14** in [lot-records §9.1.2](../specs/lot-records.md), archived the complete
question/follow-up as MATCHES, and regenerated the question index. Added the non-Write method
inventory in §9.3 and six normative conformance vectors **T74–T79** in §17.8. These are prose
requirements, not executable tests or measured game results.

The native explanation is specific: CreateDustClouds acquires the effect and stores O at
construction+0x13C. Shutdown removes O+0xC from the occupant manager and releases O at
0x005D0F96, leaving the field unchanged. Effect destruction reaches 0x005E8C40 with O+0xC;
the base destructor installs the reported bare Serializable vtable AA41BC at O+0x10. A later
field-13 adjustment therefore points into stale teardown storage. This does not create a
temporary bare occupant or change the live-effect contract. The export's vector-destructor
name at 0x005E8C40 is misleading; instruction bytes establish the vtable stores and argument.

The spec separates three cases: valid live derived effect, valid supplied bare Serializable
that Read rejects to null, and stale storage on which interface calls have no valid contract.
It accounts for helper QI, construction ownership, callback borrowing and independent observer
pins, including a legitimate final pin release after natural owner shutdown. It preserves the
reported mismatch and forbids guessed interfaces, a fixed four-byte adjustment, suppressed
warnings or leaked pins.

**Run 470's timing and later crash remain unproven.** The identity pattern is explained by a
native teardown path, but its occurrence in that run needs correlated construction Write,
Shutdown, effect destruction/base-vtable installation, helper argument, callback nesting and
observer-acquisition traces. §9.1.2 specifies this passive observation without dereferencing
known-dead storage. Existing LR13 results remain distinct: turbo run 470 failed with construction
107/108 and a later foreign exception; normal run 471 passed with construction 169/169.
This lane made no game request and claims no new exactness or runtime fix.

§9.3 inventories all exported named construction operations beyond Write, allocation/destruction,
interface adjustment/refcount thunks and inherited occupant/property-holder surfaces. Known-live
position, bounds and class getters are read-only calls; QI has balanced lifetime effects; all
simulation, initialization, loading, placement, drawing, rotation/lighting and teardown methods
require passive observation of natural calls. Draw and tick(0) are not pure probes. Write
exactness does not compare any of these methods or establish an exhaustive city census.

The requested Owner closure table is not in glue-construction-lr13's lane report. The public
core-construction5b report locates it in an engine module README; the explicit prohibition on
reading engine/ takes precedence, so it was not accessed. The native inventory is provided
for the coordinator to reconcile with that table. No engine/plugin/capi source was read or
edited; ABI tables were read only, and no coverage map or tool was edited. Engine changes already present
in the required LR13 merge are inherited, not authored here. No ABI-row proposal is needed.

Evidence came from main-checkout re_query export queries, instruction bytes and bounded
construction decomp reads; no REA call was attempted because the brief says no binary is open.
Milestones: required LR13 merge **c6baf4b15**, teardown spec **522470bed**, inventory/vectors/
question closure **c442cabcc**. Git reported packed-refs.lock permission diagnostics during
merge/commits, but each operation created its expected commit; shared refs were not changed.

Validation: **questions check PASS; doccheck check PASS; git diff --check PASS**.
Cargo/clippy/CTest are n/a for this RE-only change. LR6, LR7 and LR12 were not part of this
LR14 assignment and remain open. No asynchronous operation or game label is outstanding.

## Handoff
- Ready for coordinator review/landing; supervisor publishes detached HEAD to the reserved branch.
- Next implementation measurement: passive lifetime attribution under turbo before full construction closure credit.
- Keep the historical failed turbo verdict distinct from normal-speed exactness; no default-enable recommendation.
- Merge queue uses actual full HEAD, kind re, abi false, game_runs empty.
