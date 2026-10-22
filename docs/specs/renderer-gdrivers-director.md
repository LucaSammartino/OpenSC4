# Spec: graphics-driver COM director (`cSC4GDriversCOMDirector`)

Status: **implementation spec for the 65-body / 8,068-byte attributed inventory and director dispatch**.
Clean-room behavior contract for SimCity 4
Deluxe GOG 1.1.641. Prose, constants and address citations are the implementer-facing output; original
program material must not be consulted by implementers.

## 1. Scope and exact attribution

This owner publishes factories through the COM director protocol. It is separate from the rendering
operations in [renderer-driver.md](renderer-driver.md). No new coverage of `nSGLDX7`, `nRZSimGL`,
`cGZBufferSimGLCache`, or RD3–RD5 is claimed here.

The coverage-attributed inventory is **65 bodies, 8,068 bytes**. The two directly named methods
account for only **85 bytes**; they are not the completion denominator. The remaining **63 bodies,
7,983 bytes** include the first two registered factories, their constructors/destructors, interface
thunks, OpenGL operations, and software-driver support. These are real reachable bodies, retained in
scope even where their export names misleadingly mention DX7. A folded deleting destructor also names
this class among eight aliases; its 30 shared bytes are supporting dispatch evidence outside the 8,068.
The singleton accessor and inherited methods are likewise additional supporting evidence, not a
substitute inventory. No native object layout is prescribed.

The exact checklist is in [the attributed-body inventory](renderer-gdrivers-director-inventory.md).
It preserves the current attribution, including neighbor rules; neither ownership nor the coverage
denominator is changed. In particular, the constructor really passes factory addresses
`0x0087E1D0` and `0x008DCD30` to registration (`0x00466265`, `0x00466277`); their calls to
`0x0087DEE0` and `0x008DC960` are executable calls, not false constant references. Those constructors
install the tables whose methods lead to the remaining bodies. The separate factory
`0x00886700` and capped DX7 driver are not added to this inventory.

| Attributed body | Address | Export size | Checklist / contract |
|---|---|---:|---|
| Constructor | `0x00466250` | 79 | specified, §2: base construction and three registrations |
| `GetDirectorID` | `0x004662A0` | 6 | specified, §2: `0x64B2A903` |
| Folded deleting destructor, alias of `icf::cGZDrawCOMDirector::~cGZDrawCOMDirector` | `0x00989DA0` | 30 shared | specified, §6.5; counted once as shared evidence |

The primary vtable at `0x00A87EE8` has 17 slots. Only slot 13 targets a body exclusively named under
this director; slot 16 uses the folded derived deleting destructor. The constructor also installs a
secondary interface table at `0x00A87EB8`.
The complete slot checklist and secondary dispatch audit are in §6. The existence of
this table does not imply additional driver APIs, device enumeration, or a director-specific renderer
start/stop protocol.

Evidence: export function inventory, `decomp/index.tsv`, `decomp/cSC4GDriversCOMDirector.c`, folded
alias inventory at `0x00989DA0`, and instructions at `0x00466250–0x0046629E` / `0x004662A0–0x004662A5`.

## 2. Construction, identity and lazy access

The exposed logical state comprises a reference counter, a library-path string, service and framework
handles, an ordered child-director collection, and a class registry.
The base constructor sets the reference counter to zero, constructs an empty path and registry,
clears the service/framework handles, and creates an empty child collection (`0x0090E18D`).

The derived constructor then installs this director's interface dispatch and makes these three
registrations, in this order. All use the **no-argument object factory** registration kind (tag 1),
not the alternate factory kind that receives an interface ID and output argument.

| Registration order | Class ID (32 bits) | Factory entry | Distinguishing identity |
|---:|---|---|---|
| 1 | `0xC4554841` | `0x0087E1D0` | first driver factory; the getter at `0x0087E0A0` returns this ID, despite its misleading DX7 export name |
| 2 | `0x7ACA35C6` | `0x008DCD30` | second driver factory; getter `0x008DA490` |
| 3 | `0xBADB6906` | `0x00886700` | DX7 driver factory; getter `0x00886E00` |

Registration stores a class ID, a factory and kind 1 with unique-key insertion (`0x0090E133`). It does
not invoke a factory. A duplicate class ID preserves the existing factory and kind; it does not replace
them or create another enumeration entry (`0x0090DF37–0x0090DF40`, `0x0090DF6D–0x0090DF7B`).
The derived constructor does not inspect registration results, undo earlier
registrations, initialize a device, or query a service. On ordinary completion it returns the director.
Allocation exceptions are not converted into a Boolean failure by these bodies. Registry insertion
and iteration order are specified separately in §5; registration order is not a promise of enumeration
order. `GetDirectorID` returns exactly `0x64B2A903`, independently of initialization and references.

The accessor `GetSC4GDriversCOMDirector` (`0x00466440`, 58 bytes, not an owner-attributed method) returns
one static director identity. On its first call it sets bit 0 of a static guard **before** constructing
the director, then registers an exit callback and returns the static identity. Subsequent calls with
that bit set return the same identity without reconstruction, re-registration or a reference increment.
There is no lock or atomic guard operation in this body. Reentrant access after the bit is set can
therefore return the identity before construction finishes. If construction unwinds, this body provides
no guard reset/retry path. These statements describe the native accessor, not a requirement to
reproduce unsafe concurrency in a portable implementation.

Numeric/order evidence: registration calls `0x00466272`, `0x00466284`, `0x00466296`; class-ID getter
instructions at `0x0087E0A0`, `0x008DA490`, `0x00886E00`; kind write `0x0090E150`; guard store
`0x0046645C` precedes constructor call `0x00466462`, exit registration `0x0046646C`.

## 3. Initialization and service binding

`InitializeCOM(service, path)` is the inherited body at `0x0090DE57` (75 bytes). A null service returns
false before changing the director. A nonnull service performs these operations in order:

1. Remember the service handle, without `AddRef` or release of a previous handle.
2. Call that service's slot 4, with no explicit arguments, and remember its result as the framework
   handle. A null returned framework does not cause initialization failure.
3. Copy the supplied **source string interface** into the director's path by value. This is not a
   pointer to a C string. The string assignment wrapper calls its copy operation, which reads source
   slot 7 (length) before source slot 6 (character data), then assigns that many bytes. Embedded zero
   bytes are not a terminator for this operation.
4. Call each child director's `InitializeCOM` in child order with the **same service and original path
   arguments**. Disregard each child's Boolean result.
5. Return true.

The path is not the service pointer: instruction argument tracing at `0x0090DE6E` and `0x0090DE82`
disambiguates the unreliable pseudocode. Repeated successful calls repeat the service query, overwrite
the remembered handles and path, and revisit children. A later null-service call returns false and
retains the last successful state. This director starts with no children and adds none in its derived
constructor. No factory is invoked by `InitializeCOM`.

A null source path with a nonnull service is not a checked failure: the source string's virtual table
is dereferenced after the service/framework handles have been written. There is no rollback here.
A valid source string is therefore a precondition for ordinary initialization. The null-service branch
does not inspect the path at all. Path-copy evidence: string table `0x00A80810`, assignment wrapper
`0x0090E230`, copy body `0x0090E20A–0x0090E22F`, length-based destination assignment `0x0090F546`.

### 3.1 Registration with the containing director

The native caller at `0x004600D6–0x004600EB` obtains the graphics director, passes it to the containing
director's shared `AddDirector` helper, then calls graphics-director primary slot 4 with a service
argument. That last method is the one-argument true-returning no-op in §6.1; it does not create or
start a driver. No further caller semantics are required to interpret these calls.

`AddDirector(parent, child)` (`0x0090E0D4`, 67 bytes) initializes the supplied child with the parent's
remembered service and the parent's path string interface, disregards the result, recursively adds
each of the supplied child's children to the **same parent**, then appends the supplied child itself
to the parent's collection. Thus descendants are appended before their ancestor. This is not merely
a retain of a nested subtree. There is no child `AddRef`, deduplication, null check or cycle guard in
this helper. Appending is confirmed by `0x008FF785` and its insertion slow path. The graphics director
itself has no children after construction; recursive behavior matters only when this shared helper
is used with an existing child collection.

## 4. Class lookup and factory ownership

`GetClassObject(classID, interfaceID, output)` is inherited (`0x0090DFAC`, 127 bytes):

1. Ask children in stored order using the same three arguments. Stop on the first child whose Boolean
   return is true. A child returning false does not prevent later children or the local registry lookup;
   the parent does not restore an output that a child changed.
2. If no child succeeds, look up the exact 32-bit class ID in the local registry. A miss returns false
   without a local output write.
3. For kind 1, invoke the registered no-argument factory. Immediately call the returned interface's
   slot 0 (`QueryInterface`) with the requested interface ID and caller's output. On success return true
   with no additional retain or release by the director. On failure, call slot 1 (`AddRef`) and then
   slot 2 (`Release`) on the factory result, and return false.
4. The shared body also supports kind 2: pass the requested interface ID and output directly to its
   factory and return its Boolean result, without the kind-1 ownership sequence. Other kind tags
   return false. None of the three derived-constructor registrations uses kind 2.

**Failure quirk:** kind 1 has no null check between factory return and virtual `QueryInterface`
(`0x0090DFFC–0x0090E00B`). A null factory result reaches an invalid dereference, rather than a clean
false return. A portable implementation may report an allocation failure safely, but must not describe
that as the shipped body's Boolean failure behavior. Unsupported interface failure, in contrast, takes
the explicit `AddRef` then `Release` sequence (`0x0090E015–0x0090E020`).

The three registered wrappers allocate and construct a fresh driver on every call; the director
contains no driver-instance cache. Each returns an adjusted base-interface identity when construction
succeeds, or null if allocation or its constructor returns null. The second wrapper supplies an explicit
zero constructor argument; the first and third supply none. No wrapper calls `QueryInterface`, retains
the result, enumerates adapters, or opens a device. Construction internals belong to the driver owners.
Wrapper evidence: `0x0087E1D0–0x0087E1F2`, `0x008DCD30–0x008DCD54`, `0x00886700–0x00886722`.

## 5. Class enumeration

The inherited `EnumClassObjects(callback, context)` (`0x0090E02B`, 86 bytes) visits all children in stored
order first, passing the same callback and context to their slot 5. Then, for each local registry entry,
it invokes `callback(classID, 0, context)`. Neither child nor callback return values stop the traversal;
the method has no meaningful Boolean result. It enumerates class registrations, not constructed driver
instances, hardware adapters, or display modes.
Evidence: child calls `0x0090E043`; callback arguments/call `0x0090E05E–0x0090E068` and unconditional
iterator advance `0x0090E071`.

For the director as constructed, the local order is exactly:

| Callback order | Class ID | Unsigned ID modulo 193 | Registration position |
|---:|---|---:|---:|
| 1 | `0x7ACA35C6` | 40 | 2 |
| 2 | `0xC4554841` | 113 | 1 |
| 3 | `0xBADB6906` | 179 | 3 |

This order must not be replaced by insertion order or ascending numeric ID order. The registry
constructor requests 100 buckets (`0x00969AE3`); its initializer uses the first table size at least
100 (`0x0097C565`, `0x0090DF7C`, unsigned lower-bound comparison `0x0079FAB4–0x0079FAC8`). The native
size table at `0x00A829B0` begins 53, 97, **193**, 389. Insertion and lookup use unsigned division of
the entire 32-bit class ID by the bucket count (`0x0090DF24`, `0x00968CEE`). No resize occurs for these
three entries: the insertion resize request is element count plus one, and requests no larger than
the current bucket count return immediately (`0x0090E084–0x0090E089`, `0x009978C2–0x009978C5`).

Iteration selects the first nonempty bucket from low index to high (`0x0097C081`), follows the bucket's
node chain, then scans later buckets (`0x00969127`, `0x00879A0F`). Different keys in one bucket are
inserted at its head, so absent a resize their relative enumeration order is reverse insertion order.
The default three entries occupy different buckets. This description supplies the observable order;
it does not prescribe a native registry layout. Mutation during callbacks is not snapshot-protected,
and no concurrent/reentrant mutation guarantee is established by these bodies.

## 6. Shared dispatch and destruction checklist

### 6.1 Primary table: all 17 slots

All entries were checked against the executable's table at `0x00A87EE8` and the target instructions.
Generic folded export labels such as `operator.new` or `cFreshPlayer::SetVolumeNoLock` do not describe
the method's role on this director. Slot numbers below are primary-interface numbers, not proposed
new ABI names. Boolean returns refer to the low byte; unused upper return-register bits are not a
32-bit status-code contract.

| Slot | Target | Bytes | Complete behavior / section |
|---:|---|---:|---|
| 0 | `0x0090DE02` | 65 shared | QueryInterface, §6.2 |
| 1 | `0x0094C4B1` | 7 shared | increment reference counter modulo 2³² and return the new 32-bit count |
| 2 | `0x0090E242` | 5 shared | dispatch to primary slot 10; return that result; no deletion |
| 3 | `0x0090DE57` | 75 shared | InitializeCOM, §3 |
| 4 | `0x0091559A` | 5 shared | accept/ignore one argument; return true; no side effects |
| 5 | `0x0090E02B` | 86 shared | EnumClassObjects, §5; no Boolean result contract |
| 6 | `0x0090DFAC` | 127 shared | GetClassObject, §4 |
| 7 | `0x0090DEA2` | 43 shared | CanUnloadNow, §6.3 |
| 8 | `0x00648F00` | 3 shared | no explicit argument; return true; no side effects (export name `operator.new` is misleading here) |
| 9 | `0x009C32AC` | 4 shared | return current 32-bit reference count, without changing it |
| 10 | `0x0090DE43` | 15 shared | decrement reference count iff nonzero, return resulting count; no destruction or releases |
| 11 | `0x009CDC23` | 4 shared | return the remembered framework handle; initially null |
| 12 | `0x009D858F` | 4 shared | return the remembered service handle; initially null; no retain |
| 13 | `0x004662A0` | 6 exclusive | director ID `0x64B2A903`, §2 |
| 14 | `0x0090DECD` | 28 shared | GetLibraryPath, §6.4 |
| 15 | `0x0090DE52` | 5 shared | GetHeapAllocatedSize: delegate to `0x00949B47`, which returns 32-bit zero; does not measure registry/driver allocations |
| 16 | `0x00989DA0` | 30 shared/folded | deleting destructor, §6.5 |

These are **16 shared target bodies, 506 bytes**, plus the exclusive 6-byte ID getter. The constructor
is not a vtable entry. This dispatch inventory must not be reported as 17 new exclusive owner bodies.

### 6.2 Interfaces and secondary callbacks

For a valid director and writable output, QueryInterface recognizes exactly these three IDs:

| Requested IID | Returned identity | Reference effect |
|---|---|---|
| `1` | primary identity | one primary AddRef |
| `0xA21EE941` | same primary identity | one primary AddRef |
| `0x03FA40BF` | secondary identity | one primary AddRef |
| anything else | no output write, return false | none |

Successful queries return true. Failed queries preserve an existing output sentinel; they do not
clear it. Successful queries with a null output pointer are not validated and dereference that pointer.
Both supported identities denote the same director and use the same reference counter. Evidence:
32-bit comparisons `0x0090DE05–0x0090DE1B`, output writes `0x0090DE2F` / `0x0090DE36`, virtual AddRef
`0x0090DE3A`. Failure returns before any output write or AddRef.

The executable's secondary table at `0x00A87EB8` has 11 nonnull entries (slots 0–10), followed by a zero
word. It exposes no additional class registry or per-driver state:

| Secondary slots | Target(s) | Bytes | Complete contract |
|---|---|---:|---|
| 0 | `0x00460F50` | 8 shared | adjust to primary identity, then QueryInterface |
| 1 | `0x00460F60` | 8 shared | adjust to primary identity, then AddRef |
| 2 | `0x00460F70` | 8 shared | adjust to primary identity, then Release |
| 3–10 | `0x00949ADE` | 3 shared once | each returns true with no explicit arguments and no side effects |

Thus every published secondary lifecycle callback is a successful no-op. This director has no custom
pre/post initialization, start, stop, frame, or configuration work hidden in those callbacks. This
statement is bounded to these eight entries; no meaning is invented for an individual callback slot.
It must not be replaced by the unrelated `cRZCOMDllDirector::PreFrameWorkInit` body at `0x0087B1FC`,
which is **not** referenced by this director's secondary table.

### 6.3 References and unload decision

AddRef increments a 32-bit unsigned counter. RemoveRef/Release saturate at zero; reaching zero does
not destroy the director. Example: new director → Release returns 0 → AddRef returns 1 → Release
returns 0; the registry and path survive throughout. Counter overflow wraps (`0x0094C4B1`); decrement
tests only for zero (`0x0090DE43–0x0090DE4E`). No atomic reference operation is present.

**Native unload quirk:** CanUnloadNow returns true immediately when its own reference count is
**nonzero**, without consulting children. At zero it asks children in order and returns false at
the first false response; otherwise true. A childless graphics director therefore reports true at
both zero and nonzero counts. It does not inspect created-driver references, a live-instance count,
service availability or device state (`0x0090DEA6–0x0090DECC`).

### 6.4 Path and configuration access

GetLibraryPath(destinationString) calls destination slot 3 with the stored character buffer and its
exact byte length, then returns true, disregarding the destination operation's result. It does not
add a terminator to the supplied length or normalize the path. Initially the length is zero. A null
destination is not checked. This is a copy, not a transfer of the director's string storage. Evidence:
length subtraction `0x0090DED6–0x0090DED9`, destination call `0x0090DEE0`.

The service/framework getters return the remembered handles only; they perform no service discovery.
The constructor, primary no-op slots and all secondary callbacks contain no configuration parsing,
adapter preference selection, or display-mode enumeration. Class enumeration (§5) is the director's
actual enumeration surface. Such rendering configuration belongs to other owners.

### 6.5 Destruction and singleton exit

The folded deleting destructor always runs base destruction first, then frees the director's storage
iff **bit 0** of its deletion argument is set; other bits alone do not trigger a free. It returns the
original identity (`0x00989DA0–0x00989DBD`). Releasing a reference is a different operation and never
dispatches here through this director's Release/RemoveRef pair.

Base destruction (`0x0090E0A5`, 47 bytes) switches to base dispatch, destroys the class registry,
frees the child-pointer collection storage if present, and destroys the path string, in that order.
Registry destruction frees each registry node and the bucket storage; these nodes contain factory
addresses, not owned live drivers. It does not invoke factories, destroy previously returned drivers,
Release children or services/frameworks, or call the secondary stop callbacks. Supporting native
cleanup: `0x0099784E`, node clear `0x0097C1A2`, collection deallocation `0x0090E0C5`, string destruction
`0x00414250`. The misleading folded/template labels on these helpers do not imply font resources.

The singleton's registered exit callback (`0x00A7AE30`, 10 bytes) calls **base destruction directly**
on the static director. It does not use the deleting destructor, free static director storage, or
clear the accessor's constructed bit. Consequently accessor use after this exit cleanup is not a
supported recreation path; the native guard would still bypass construction.

### 6.6 Supporting-body checklist and boundaries

Every row below was checked in native instructions. These are dependencies of the contract above,
not additions to the two directly attributed owner bodies.

| Supporting body | Address | Checked behavior |
|---|---|---|
| Singleton accessor / exit callback | `0x00466440` / `0x00A7AE30` | §2 / §6.5 |
| Base constructor / destructor | `0x0090E18D` / `0x0090E0A5` | §2 / §6.5 |
| AddDirector / child-pointer append / append growth | `0x0090E0D4`, `0x008FF785`, `0x00994D77` | §3.1; recursive registration, then append |
| AddCls kind 1 / kind 2 | `0x0090E133` / `0x0090E160` | §2 / §4; latter unregistered by derived constructor |
| Three class-ID getters | `0x0087E0A0`, `0x008DA490`, `0x00886E00` | §2 constants |
| Three factory wrappers | `0x0087E1D0`, `0x008DCD30`, `0x00886700` | §4, fresh construction and null branches |
| Registry constructor / bucket initialization / size selection | `0x00969AD5`, `0x0097C55A`, `0x0090DF7C`, `0x0079FA90` | §5; 100 requests 193 buckets |
| Unique insertion / insertion without resize | `0x0090E081` / `0x0090DF0C` | §2 / §5; preserve duplicate and head-insert new key |
| Registry resize / lookup | `0x009978AF` / `0x00968CD9` | §5; no default resize, unsigned bucket remainder |
| Begin iterator / advance / next bucket | `0x0097C081`, `0x00969127`, `0x00879A0F` | §5 order |
| Registry destruction / node clear / deallocator | `0x0099784E`, `0x0097C1A2`, `0x0090CF63` | §6.5; no interface release |
| String construction / assignment / copy / byte assignment / destruction | `0x00578390`, `0x0090E230`, `0x0090E20A`, `0x0090F546`, `0x00414250` | §§2–3, §6.5; storage owned by director |
| String length / character-data getters | `0x0090E4B6` / `0x009D7E63` | §3; length excludes terminator, data getter returns stored buffer |
| Constant zero helper | `0x00949B47` | primary slot 15, §6.1 |

All director primary/secondary slots above are specified. Round 1's stronger assertion about all
attributed bodies was incorrect: the reachable 7,983 bytes require the additional contracts and
checklist linked in §1. No completion of the capped DX7 driver, allocator/string/container general
APIs, containing `cSC4COMDirector`, or other directors is implied.

## 7. Independent discriminating vectors

These are normative paper vectors derived from native control flow, not claims of executed tests.
Observe call logs, output sentinels and logical state independently of any implementation's internals.

| ID | Stimulus | Required result / distinguishing observation |
|---|---|---|
| GD1 | First accessor call, then two more | One construction, three registrations in §2 order, one exit registration; identical identity each time; accessor does not increment references |
| GD2 | Construct without initializing COM | Zero factory calls and service calls; all three class registrations exist; director ID is `0x64B2A903` |
| GD3 | Initialize with null service and output/path state recorded | False; no service query, path assignment or child call; state retained |
| GD4 | Initialize with service S returning null framework, path P | True; query S slot 4 once, remember null framework, assign P; no driver factory call |
| GD5 | Initialize S1/P1, then S2/P2, then null/P3 | First two true, last false; remembered service/path remain S2/P2; no service retain/release calls |
| GD6 | Request missing class with output sentinel X and no children | False; X unchanged; no driver construction |
| GD7 | Request each registered class using a successful fake kind-1 factory/interface | Matching factory only, then `QueryInterface(interfaceID, output)`; true; no director-side `AddRef`/`Release` after success |
| GD8 | Existing class, factory interface rejects requested IID without writing output | False, sentinel retained; order is factory → QI → AddRef → Release, distinguishing it from Release-only cleanup |
| GD9 | Existing kind-1 class, factory returns null | Native invalid dereference before any Boolean result; must not use this case to infer a clean false-return rule |
| GD10 | Children A and B: A returns false and writes Y; B returns true and writes Z | Calls A then B; true with Z; no local factory; parent performs no output clearing |
| GD11 | Enumerate with callback returning false on its first call | All local classes still reported once: `7ACA35C6`, `C4554841`, `BADB6906`; each with second argument 0 and original context; zero factory calls |
| GD12 | Two successful requests for the same local class | Two factory invocations; no director cache or initialization prerequisite |
| GD13 | QI primary IID 1, then secondary IID `03FA40BF`, then `A21EE941`, starting at zero refs | True three times; first and third identities equal, second is the secondary identity; shared counter becomes 3 |
| GD14 | QI unsupported IID with sentinel X, starting at 3 refs | False; X retained; refs remain 3 |
| GD15 | On secondary identity call AddRef then Release, starting at 3 refs | Counts 4 then 3, visible through primary slot 9; no distinct secondary counter |
| GD16 | Release a fresh director twice; acquire/release one ref; enumerate afterward | Counts 0, 0, 1, 0; no destruction; all three registrations still enumerable |
| GD17 | Counter `FFFFFFFF`, AddRef, then RemoveRef (shared-counter boundary fixture) | AddRef returns 0 by 32-bit wrap; RemoveRef returns 0; no free |
| GD18 | CanUnloadNow at zero and nonzero refs on fresh childless director | True in both cases |
| GD19 | Shared child fixture A returns false from CanUnloadNow; B would return true | At zero refs: false, only A called. At nonzero refs: true, neither child called |
| GD20 | Invoke primary slots 4 and 8, then secondary slots 3–10, before and after initialization | Each returns true; no handle/path/refcount/registry changes, no factory or service calls |
| GD21 | GetLibraryPath before initialization, then after copying a 3-byte path `A,00,B` | Destination slot 3 receives length 0 first, then exactly 3 bytes `A,00,B`; method returns true even if destination reports failure |
| GD22 | Initialize with S returning F; mutate the source path object afterward | Primary slots 12/11 return S/F without retain calls; stored path retains the copied bytes |
| GD23 | Initialize with nonnull S and null path | Service query and handle writes precede native invalid dereference; no false result/rollback is established |
| GD24 | Initialize parent with children A (returns false) then B (returns true), valid S/P | Both receive original S/P in order; parent returns true; no child-result rollback |
| GD25 | Register an existing class ID again with a different factory, shared helper fixture | Original factory/kind retained, one enumeration entry only; duplicate factory never invoked by registration |
| GD26 | Enumerate child fixture A then B before default local entries, all callbacks return false | Child delegations A then B, followed by local IDs `7ACA35C6,C4554841,BADB6906`; no early stop |
| GD27 | Local class kind-2 fixture returns false and writes Y | Direct call with IID/output, false/Y propagated; no kind-1 QI/AddRef/Release sequence |
| GD28 | Destroy independent director with deletion flags 0, 2, 1, 3 (separate instances) | Base cleanup in every case; object storage freed only for 1 and 3; no driver, child or service Release |
| GD29 | Invoke singleton exit callback after holding refs and previously creating a driver | Base container/path cleanup only, regardless of director refs; no static-storage free, driver destruction or secondary stop callback |
| GD30 | Add child C, whose existing children are A then B, to P with nonnull service and valid path (shared helper fixture) | C initialization first (which visits A then B); recursive additions initialize A then B again; P appends A, B, C, with no AddRef |
| GD31 | Primary slot 15 before/after registration, initialization and factory requests | 32-bit zero each time; no allocation-size computation |

Fixtures with children, duplicate registration, alternate factory kinds or boundary counters exercise
the shared native bodies; the derived constructor does not supply APIs to configure those cases.
Allocation failure in a factory wrapper itself returns null and skips its constructor. Passing that
result through kind-1 GetClassObject is the separate, unchecked failure in GD9. Invalid pointers and
concurrent registry mutation do not have additional safe-failure guarantees in this contract.
