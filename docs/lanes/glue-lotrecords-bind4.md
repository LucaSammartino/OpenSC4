| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotrecords-bind4: native lot binding | `glue-lotrecords-bind4` / engine lane, detached HEAD | wip: findings `12382292`; cargo 4641/0, clippy 0 diagnostics, CMake 208/208, ctest 68/68; LR10/LR11 filed; no plugin change or game credit | 1. Specify concrete lot SetPopulation and complete SetState. 2. Implement developer-inclusive lot/unit/wall insertion and native scenarios, then own/union 2032/ref0. |

## Result

Documentation-only WIP finding. `12382292` commits the exact concrete-caller blockers and questions
before call 30. No native implementation or activation is claimed. This round used one guarded session,
no subagents, restart, merge, ref update, push or external communication.

## Boundary audit

The LR8/LR9 answers are accepted. Primary lot slot 3 returns the borrowed property holder at +8;
slot 4 returns the borrowed traffic source at +0x1C. The secondary views are not new QI identities.
QI succeeds only for IID 1, lot and Serializable, acquiring one common-owner reference; failure preserves
the output sentinel. No ABI row or C ABI export was changed in this lane.

### Concrete insertion and ownership

`engine/plugin/FC4ZoneDevHost.cpp` NewLot already invokes the published pool allocation and five-word
constructor through the direct-call compatibility layer. Its result is a native lot. Replacing that
callback with a portable-record pointer would require a complete concrete lot implementation, rather
than another native construction call. `cFC4ZoneDeveloper.h` still has an older Stage A comment;
the inspected executable host code has the construction path. No source comment cleanup was made.

- New native lot count is zero. The caller output and master list require two distinct AddRefs. Cell
  pointers borrow the master-held lot; iterator insertion must not add a third owning reference.
  Removal clears borrowed cell pointers and releases the master hold; caller output is independently held.
  Last native Release calls Shutdown and the property-view deleting destructor, returning native pool
  storage. A foreign allocation cannot use that destructor. A replacement must provide its own destruction
  and preserve both holds, including partial insertion failures; no such unverified object was installed.
- Native business-unit construction starts at count zero. Unit nodes retain member lots, and each lot's
  SetUnit retains the unit. Remove/Shutdown must break both directions. A factory creation reference
  would be an additional reference to release after membership, not a substitute for cycle teardown.
- Native walls use a distinct pool. SetLot acquires the new lot before releasing the previous lot.
  Creation owns a local wall reference, hands the occupant view to occupant-manager slot 20, and drops
  that local reference on success or property-load failure. Native success still returns AL=0.
  ClearLot/foundation/slot-62 direct helpers require developer-inclusive replacement, not wall factories.
  Refresh retains a zero-hash wall and preserves its position/size/type/lot; LR6/LR7 remain explicit host
  contract/native-undefined boundaries, with no synthetic success or null guard added.

### Narrow remaining blockers

LR10 asks for lot slot 55's complete mutation contract. Its ABI row publishes pop 8 but only
`eax-or-void`; the record format does not determine which population maps change, capacity/clamping,
zero/duplicate-key behavior, cache changes or messages. Native tract population adjustment calls it;
ReoccupyBuilding passes its AL through. A single-map insert plus `true` is not specified behavior.

LR11 asks for complete lot slot 34 owner behavior. The published message recipe explicitly leaves
UpdatePrimaryOccupantWealth unspecified, and does not supply all population/resource/state-day effects
and their order before the queued copy. Native SetState copies a native lot and embedded containers;
delegating it on a replacement pointer would assume an unpublished full native layout/construction
contract. The save record and four view corrections are insufficient to implement that operation.

The traffic-list container is **not** a missing contract: allocators.md publishes 0x34-byte B-allocated
trip nodes and B route buffers, and traffic-simulator.md owns the trip behavior. GetPopulations similarly
exposes native containers; Rust Vec/BTreeMap storage cannot be returned directly. These are concrete
adapter obligations, not justification for an unsupported cast.

### Verification disposition

T26-T30 were reviewed as requirements: independent view offsets/QI sentinel and reference counts;
load factory versus direct runtime construction; unit creation versus membership holds; direct wall
helper traces and their AL=0 success. They are not claimed as executed native scenarios in this lane.
No disabled wrapper, independent factory, lotrecords option, default, map, tools or plugin change was
added. No runtime shadow, concrete replacement, focused native ABI/ownership test, or game credit is
claimed. Such tests must exercise an implemented object after LR10/LR11, not a mock of invented behavior.

No game request was queued: there is no candidate DLL change to exercise. Actual creation/lookup,
traffic/property access, save/reload, destruction with owner-path evidence, own 2032/ref0 and literal
39-key union plus candidate remain requirements for a future supported binding. Idle runs would not
resolve the missing contracts. No requests outstanding; no artifacts or game files committed.

### Measured gate and delivery

One baseline `qbuild.py --wt <WT> all` passed: cargo 4,641 passed / 0 failed / 0 ignored,
clippy 0 diagnostics, CMake 208/208 edges (104.2 seconds), full ctest 68/68 (125.4 seconds).
Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
The wrapper noted the tree changed during the run: only the question/lane documentation was edited
and committed while it ran; engine/build inputs stayed unchanged. This is a baseline measurement,
not a game-tested candidate DLL or an immutable final-tip gate record. No Rust edits; fmt n/a.
`git diff --check` passed. The stable change is docs-only, so no additional engine gate was needed.

Questions LR10/LR11 were appended only to `docs/specs/questions/lot-records.md`; LR6/LR7 unchanged.
The coordinator must regenerate the question index using its permitted spec/tooling workflow; this
engine lane cannot run tools/ or edit the generated index. No ABI exports or metadata changed.
Queue disposition: kind docs, state wip, no game runs. The runtime task remains unfulfilled pending
the specified contracts, implementation and independent ABI/ownership/core plus real-game verification.

Environment: q.py Bash startup failed E_ACCESSDENIED; capped PowerShell reads and quiet qbuild worked.
Git emitted a packed-refs.lock permission diagnostic but the detached milestone commit succeeded;
no ref/config/permission changes were attempted. Guard manifest:
`C:/Users/tlori/FreeCity4/build-runs/run4-glue-lotrecords-bind4-r1-manifest.json`.
