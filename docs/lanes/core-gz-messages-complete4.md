| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gz-messages-complete4: portable GZ messaging completion | `core-gz-messages-complete4` / (engine lane, detached HEAD) | wip: core 19269ab9, tests 7feda760, audit d60acdea, checklist 8cc85506, final correction fac8c016; cargo 4662/0, clippy 0, ctest 68/68, fmt 7 PASS; 35 GZ tests covering V01-V14; GZ8-GZ10 open; ABI unchanged; no game/shadow/replacement or complete-owner credit | 1. Resolve GZ8-GZ10 and reconcile the full owner body inventory. 2. Preserve logical round 2/3, one completion round remains. 3. Coordinator regenerates the questions index. |

## Report

Implemented the portable behavior published for GZ1-GZ7 in `framework-gz-services.md` sections 2-4A and checked the independent V01-V14 expectations. Whole-owner completion remains **incomplete**: GZ8-GZ10 request cleanup ordering, base-service contracts and the missing complete per-body inventory. No owner/coverage, shadow, replacement or default promotion follows from these unit tests.

Changes in the seven existing `engine/core/src/gz_messages/*.rs` files:

- Added the server-2 shared unsigned count, wrapping AddRef and nondeleting positive-only RemoveRef. It remains distinct from posted count and the 64-bit taken counter.
- Replaced the latched queue signal with an empty-to-nonempty pulse callback. Blocking waits convert unsigned microseconds to milliseconds, preserve infinity, map native OS results and retry after timeout/failure without resetting or joining. Queue destruction closes the host signal before releasing messages, including the embedded server-2 queue.
- All equality methods call the queued receiver and test only the low byte. Gets now report caller AddRef followed by queue Release, preserving observable calls instead of reducing them to a net transfer.
- Registry operations expose AddRef/GetType/destructor re-entry, unlink before Release at positive depth, preserve still-linked flush replay and the per-generation process-wide guards. Generation 2 includes pending construction temporary holds. Generation 1 distinguishes counted pending adds from raw removes and preserves transfer/duplicate-add leak during direct apply.
- Server 2 keeps every per-occurrence snapshot hold through all deliveries, then cleans up forward. Wildcard lookup occurs after typed-list snapshot acquisition, including safe nested-send pending application to the separate wildcard list. Server 1 retains its 385-target policy, exact-type send and per-callback snapshot cleanup.
- Directed slots are void. Generation 1 copies records and borrows targets without reference calls. Generation 2 preserves temporary pair construction/copy/destruction, target-then-message assignment, identity suppression, persistent delivery locals and message-then-target final cleanup. Both preserve cancellation's defined prefix/suffix-skip witnesses and shared signed quota, including cancel/post and nested tick behavior.

## Measured verification

- Initial baseline `qbuild all`: cargo 4641 passed / 0 failed, clippy 0 diagnostics, CTest 68/68. Its native run noted concurrent tree changes, so it is baseline evidence only.
- Intermediate `quick` gates: core 2720/0 and 2722/0, dependent crates compile-only and native build PASS. The first quick caught a test fixture assuming a reference on a raw generation-1 directed target; the fixture was corrected.
- Stable full gate at 8cc85506: cargo 4661/0, clippy 0, CTest 68/68. A subsequent contract audit found the safe nested-send wildcard lookup ordering case; fac8c016 fixes it and adds an independent regression.
- **Final stable `qbuild all` at fac8c016: PASS cargo 4662/0, clippy 0 diagnostics, native build 4/4 edges, full CTest 68/68.** fc4verify 234.3 seconds (build 73.3 seconds, CTest 156.0 seconds); no tree-change qualifier. No engine source changed during this gate.
- Final `qbuild fmt` on all seven touched Rust files: exit 0. `git diff --check`: clean. GZ tests: 35 total, 21 added above the parent's 14; all fourteen published vector rows are covered, plus focused order/lifetime/boundary cases.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `qbuild-fmt.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No logs or generated ABI entries are committed.
- Game: n/a, portable core only. No new request, outstanding label, measured reference-warning count or native shadow is claimed; no plugin/CAPI/CMake/ABI/default/enable/map changes.

Questions appended only to `docs/specs/questions/framework-gz-services.md`: **GZ8** cleanup visitation/clear lifetime; **GZ9** service Release/base QI/owner-specific forwarding; **GZ10** complete 166-body attribution inventory. The generated questions index is intentionally left for the coordinator (tools/ is prohibited in this lane).

Next: obtain those contracts and reconcile every attributable owner body against the checklist below before requesting complete-owner credit. Preserve parent logical round 1/3 plus this completion phase 1/2 = logical round 2/3; only one completion round remains, no reset/fourth round. This guarded round ends here without selfrestart. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-gz-messages-complete4-r1-manifest.json`.

## Completion scope and per-body checklist

This is logical round 2/3 (parent round 1/3 plus completion phase 1/2). All fourteen published independent vectors are assertions in `engine/core/src/gz_messages/tests.rs`; they are not a game shadow. No new game request, ABI export, plugin/CAPI/CMake/default/enable/map change. No aggregate owner credit is requested.

`M` means the named portable behavior is modeled and tested, not a complete native COM body. `H` means an explicitly retained host/platform boundary. `U` means unresolved contract or inventory; completion is withheld. The published owner totals are queue 30, manager 17, server2 38, server1 81 (166 bodies). The spec does not provide every attributable body; GZ10 requests that inventory. The rows below enumerate the published entries/helpers without fabricating names or counting shared helper aliases as additional completed owner bodies.

### Queue2 (section 2)

| Entry/body | Portable implementation | Evidence/status |
|---|---|---|
| slot 0, QI (entry unspecified) | no invented base-QI behavior | U GZ9/GZ10 |
| `0093EC9F` AddRef | `MessageQueue2::add_ref` | M wrapping dword |
| `0092CFAA` Release | `release`, destroy at 0/1 | M release-at-zero/one test |
| `0092DDD2` push back | `push_back` | M queue order/ownership/pulse |
| `0092DE2C` push front | `push_front` | M queue order/ownership/pulse |
| `0092D78B` remove first type | `remove_first_by_type` | M scan direction |
| `0092D66F` remove first equal | `remove_first_equal` | M V04 receiver/low byte |
| `0092D7E6` remove last type | `remove_last_by_type` | M scan direction |
| `0092D6CC` remove last equal | `remove_last_equal` | M V04 |
| `0092D842` remove all type | `remove_all_by_type` | M count/order |
| `0092D72A` remove all equal | `remove_all_equal` | M V04 |
| `0092DB79` remove all | `remove_all` | M queue hold cleanup |
| `0092D313` front type | `front_type` | M empty/nonempty |
| `0092D34B` back type | `back_type` | M empty/nonempty |
| `0092D384` contains type | `contains_type` | M found/not found |
| `0092D3CE` peek front | `peek_front` | M caller hold, queue retained |
| `0092D414` peek back | `peek_back` | M caller hold, queue retained |
| `0092D45B` peek equal | `peek_equal` | M V04 |
| `0092D8A1` get front | `get_front` | M AddRef caller then queue Release; V03 |
| `0092D8F0` get back | `get_back` | M blocking-back and caller hold |
| `0092CFC1` blocking front | `blocking_get_front` | M refill/retry, V02/V03 |
| `0092CFF7` blocking back | `blocking_get_back` | M failed Wait then timeout then get |
| `0092F821` count | `count` | M empty/nonempty |
| `0092E27C` deleting destructor | `destroy`; allocation/free remains host | M/H close before queued Releases |
| `0092D940` get equal | `get_equal` | M V04 |
| `0092DD7E` constructor | `new`; mode-0 event supplied by host | M/H zero refs/empty queue |
| `0092DDB0` signal/queue destruction | `destroy` invokes `close_signal` before message Releases | M/H V03 |
| other owner-attributed bodies | not individually inventoried by published spec | U GZ10; no 30/30 claim |

### QueueManager2 (section 3)

| Entry/body | Portable implementation | Evidence/status |
|---|---|---|
| `0092D033` QI | `answers_iid` own IID only; counted pointer/base QI host | H/U GZ9 |
| slots 1/2 service AddRef/Release | system-service dependency | H/U GZ9/GZ10 |
| `0092DBB1` get queue | `get_queue` | M caller AddRef, absent None |
| `0092E37A` add queue | `add_queue` | M duplicate ID unchanged |
| `0092E2E3` create queue | `create_queue` + host allocation | M/H map takes sole hold |
| `0092DED7` remove queue pointer | `remove_queue` | M first ID-order match only |
| `0092DE86` remove queue ID | `remove_queue_id` | M absent false, map Release |
| `0092E3FC` remove all | `remove_all_queues` | M state/holds; U GZ8 visitation |
| `0092E714` constructor | `new` empty map; base service host | M/H |
| remaining destructor/base bodies | no complete per-body attribution supplied | U GZ9/GZ10; no 17/17 claim |

### Server2 (sections 4.1-4.7)

| Entry/body | Portable implementation | Evidence/status |
|---|---|---|
| `0092D091` QI | `answers_iid`; native counted out/base QI host | H/U GZ9 |
| `0093056F` AddRef | one shared `add_ref` | M V01; pointer adjustment host |
| `00930577` Release | no invented service deletion semantics | U GZ9 |
| `0092EA65` send | `message_send` | M V05/V07, wildcard twice, retained holds, nested wildcard lookup order |
| `0092D0BB` post | `message_post` | M ordinary queue front/back, no immediate delivery |
| `0092E795` add notification | shared registry add | M V06, snapshot AddRef re-entry |
| `0092E560` remove notification | shared registry remove | M V06 unlink-before-Release at depth 1 |
| `0092EA00` directed post (void) | `post_directed` | M temporary/copy/destroy reference trace |
| `0092E642` cancel (void) | `cancel_directed` | M V10/V11, forward pair assignment; native UB excluded |
| `0092E040` tick | `on_tick` | M worked example in both modes, V12/V13 |
| `00473000` service tick thunk | host delegates to same tick | H no native pointer wiring |
| `0092D0F4` posted count | `posted_count` | M distinct from ref/taken counts |
| `0092D11F` drain flag | `set_drain_all` | M default true and bounded draining |
| `0092D0B3` slot-12 getter | `ref_count` | M V01 shared unsigned dword |
| `0092D1FE` create message | class gate + `create_standard_message` host | M/H known/unknown class/IID tests |
| `0092EB23` constructor | `new` | M documented initial state; service/locks host |
| `0092D05B` Init | framework AddToTick at documented priority | H no plugin wiring in scope |
| `0092D076` Shutdown | framework RemoveFromTick | H no plugin wiring in scope |
| `0092DF35` flush | shared registry `flush`, process-wide generation-2 guard | M cross-instance suppression test |
| `0092E8B5` apply pending | shared registry `apply_pending` before increment | M V07/nested send; U GZ8 clear visibility |
| `007E9330` unlink/release helper | registry erase before foreign Release | M V06 |
| `005447B0` counted target assignment | explicit host AddRef calls, snapshot retained holds | M V05/V06 |
| `0092DC24` pending pop | shared flush sequencing | M replay front stays linked |
| `0092D9AF` pending erase | unlink before pending Release | M V06 |
| `0092DA64` pending clear | walk then clear pending holds | M V07; U GZ8 clear visibility |
| `009491FF` pending target cleanup | `release_target` | M construction temporaries/replay |
| `00442E40` snapshot append | retained target vector | M V05 |
| `006C6E90` snapshot vector destruction | forward cleanup after every callback | M V05 |
| `0092D1C5` directed pair construction | target then message temporary AddRef | M append trace |
| `0092D293` directed pair copy | second target/message AddRef | M append trace |
| `004E4BB0` deque append | copied owning pair before temporary cleanup | M append trace |
| `004731D0` pair assignment | acquire/store/release, target then message, identity no-op | M V09, equal-pair/erase-order tests |
| `00473180` pair destruction | message then target Release | M V09 and append trace |
| `004732E0` deque front pop | release deque message then target before callback | M V09 |
| `00473290` boundary pop | equivalent logical pop; platform deque storage not modeled | H/M lifetime behavior; native invalid iterators excluded |
| `0092E162` erase | prefix backward/suffix forward shifts | M V10/V11 and shift traces |
| `0092DFA3` directed delivery | one local pair per invocation, shared signed countdown | M V09/V12/V13 and identity suppression |
| `0092E42D` destructor | registry, pending, directed, embedded queue cleanup | M known sequence; U GZ8 type visitation |
| `0092E779` deleting wrapper | calls modeled destruction; allocation/free host | H/M |
| remaining owner-attributed bodies | shared aliases not enough to reconstruct exact owner inventory | U GZ10; no 38/38 claim |

### Server1 (section 4A)

| Entry/body | Portable implementation | Evidence/status |
|---|---|---|
| `0092ED0D` QI | own-IID predicate; +0x1C adjustment/base forwarding host | H/U GZ9 |
| slots 1/2 base forwarding | system-service dependency | H/U GZ9/GZ10 |
| `009302AC` send | exact-type only, ignore type zero, capped 385 snapshot | M no wildcard, per-callback cleanup |
| `0092ED37` post | copied 16-byte record | M posted-value order and bounded drain |
| `00930030` add | shared registry raw-add mode | M V14, append then one AddRef |
| `0092FC2E` remove | raw pending removal, no protecting hold | M focused raw-removal test |
| `00930273` directed post (void) | copied record/raw target | M V08, all four dwords preserved |
| `0092FF6F` cancel (void) | native scan policy, no reference calls | M V10/V11 and raw cancellation |
| `00472FC0` tick thunk | same portable `on_tick`; pointer thunk host | M/H |
| `0092F9AC` tick body | value dequeue + send + directed pass | M drain modes, raw V12/V13 equivalents |
| `0092ED70` posted count | `posted_count` | M |
| `0092ED9B` drain flag | `set_drain_all` | M default true/bounded drain |
| `0093035D` constructor | `new`; base service/resources host | M/H documented defaults |
| `0092F88A` flush | separate process-wide generation-1 guard | M V14 release-before-pop, raw removes uncounted |
| `00930121` apply pending | transfer first add hold, skip/leak duplicate, raw clear | M V14 exact intentional native leak |
| `009A841F` raw pending clear | no counted-wrapper destruction | M V14; U GZ8 destructor-specific use |
| `009DB1A1` raw directed construction | copied (target, four dwords) | M V08 |
| `0092EE7C` raw deque append | copied record, no AddRef | M V08 |
| `0097BF85` raw copy | copied record, no reference operations | M V08 |
| `00473110` delivery copy | local copied record | M V08 |
| `00473240` raw deque pop | remove without any Release | M V08 |
| `0092FD24` raw erase | preserve prefix/suffix scan behavior | M V10/V11 |
| `0092F90C` directed delivery | borrowed raw target, shared countdown | M raw V12/V13 equivalents |
| destructor and other owner-attributed helpers | logical clear exists; full attributable entries absent | U GZ8/GZ9/GZ10; no 81/81 claim |

### Foreign helpers required by the published gaps (no additional owner credit)

| Entry/body | Boundary/implementation | Evidence/status |
|---|---|---|
| `009D7E63` shared count getter | `ref_count` | M V01 |
| `0090D981` RemoveRef | positive-only `remove_ref`, no destruction | M V01/wrap boundary |
| `00968B60` AddRef | wrapping `add_ref` | M V01/wrap boundary |
| `0091DE0F` service constructor | count initialized to zero | M count state; other base behavior H/U GZ9 |
| `0090DA1E` unknown constructor | same shared zero count | M count state; pointer layout H |
| `009297A0` AddRef forwarding | same shared increment | M logical count, H +4/+4 forwarding |
| `0091DD89` Release forwarding | same system-service dependency | U GZ9 |
| `00473020` +8 AddRef thunk | same shared increment | M logical count, H -8 adjustment |
| `00932E65` signal constructor | host initially nonsignalled auto-reset mode-0 resource | H no OS resource created by portable core |
| `00932DA7` Signal | queue mode-0 calls host `pulse_signal` | M V03/no latch; nonzero SetEvent branch stays host |
| `00932D95` Pulse | pulse primitive host | H normalized OS result not consumed by queue |
| `00932DE0` Wait | microseconds converted before host wait, raw OS status mapped | M V02/V03/failed-wait retry; platform lock H |
| `00932E3A` handle-valid test | host resource capability; queue never tests it | H |
| `00932DCE` unused reset body | not called | H spec explicitly excludes reset from queue path |
| `00932EC8` close/free | `close_signal` callback before queue message Releases | M/H V03 |
| `00932EB4` platform cleanup | host closes event/lock/string/platform state, no waiter join | H lifetime precondition documented |

## Unsupported native preconditions and remaining work

- Do not invoke cancellation when its pre-advanced iterator can pass the new end (including matching the last/sole record). No expected crash, return time, or post-UB delivery is asserted. Safe prefix and suffix-skip witnesses are tested separately.
- Foreign callbacks must preserve live native pending/type-list/deque traversal, assignment and pop windows. Controlled destructor/AddRef/GetType re-entry is supported; arbitrary iterator-invalidating re-entry has no promised native outcome.
- Host serializes native lock domains and supplies platform resources. The portable core uses exclusive borrows, not RefCell or an emulated COM pointer layout. No cross-thread COM replacement is claimed.
- Queue must outlive blocking gets. Signal close does not join waiters or wake cancellation. Generation-1 directed target/payload pointers and pending-removal targets require live external holds.
- Preserve the server-1 duplicate pending-add leak under direct apply and the directed suffix skip; do not normalize either into a repaired algorithm.
- Resolve GZ8/GZ9/GZ10 before owner-complete credit. Base forwarding/pointer adjustments/platform resources remain host responsibilities; no plugin wiring is authorized in this lane.
