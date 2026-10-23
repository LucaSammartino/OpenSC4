# Application bootstrap directors and registry

Windows GOG 1.1.641, 2026-10-23. Addresses are executable virtual addresses;
slot numbers are zero based. Rules marked *[compat]* are observed contracts.
*[choice]* marks implementation policy. This is a bootstrap contract, not a
claim that every service reachable from a director has been specified.

## 1. Scope and related contracts

The application adds directors before its own system services during
[application §3 and §4](application.md). This document specifies the registration
and dispatch boundary for `cGZToolsCOMDirector`, `cSC4GameDataRegistry`,
`cSC4ResourceLibCOMDirector`, `cSC4COMDirector` and `cGZResManCOMDirector`.
The registry is a system service, **not** a `cRZCOMDllDirector` subclass.

Use the following contracts for the objects made available through these owners;
their algorithms are not duplicated here:

| Boundary | Contract |
|---|---|
| Tools director, its two factories, compressed input and PCM output | [tools-com-director §§2–5](tools-com-director.md); remaining decoder proof is tracked there |
| Registry's 29 public slots, callbacks, subscriptions, shutdown and vectors | [game-data-registry §§1–6](game-data-registry.md) |
| Persistence service, resource factory registry, caches and update dispatch | [persistence-manager §§1–10](persistence-manager.md) |
| Catalog/picker windows and typed property editors | [resource-catalog](resource-catalog.md), [resource-exemplar-editor §8](resource-exemplar-editor.md) |
| Resource parser and resource buffer/IO services | [sc-resources §§4,7,8](sc-resources.md) |
| Message server, queues and standard message objects | [framework-gz-services](framework-gz-services.md) |
| Renderer and automata subordinate directors | [renderer-gdrivers-director](renderer-gdrivers-director.md), [automata-managers §1](automata-managers.md) |
| Drawing/font service interfaces | [draw-context](draw-context.md), [drawing-fonts](drawing-fonts.md), [gz-graphics](gz-graphics.md) |

Evidence: exported instruction streams and native vtable words, with local
decompilation only for navigation. In particular, ResourceLib start's decompilation
misidentifies the output-pointer local; the instructions at `00664B24–00664B46`
establish the pointer test and Release below. No live game or REA execution was used.

## 2. Shared director state and interfaces *[compat]*

The four directors share base construction `0090E18D` and the methods below.
Base state starts with unsigned reference count +8 = 0, reserved word +0C = 0,
an empty library-path string at +10, null COM and heap pointers at +24/+28,
empty child-pointer vector at +2C/+30/+34 and an empty registration hash table
at +38. The concrete constructors replace the primary (+0) and hooks (+4)
vtable pointers, then populate registrations. They do not create every registered
class eagerly.

### 2.1 Method inventory and vtable slots

Primary tables each have 17 slots. Shared folded body names in the export are
not method semantics: e.g. the body called `operator.new` at `00648F00` simply
returns true. Entries below describe these receivers, not unrelated classes
sharing the instructions.

| Slot / byte offset | Contract / native body |
|---|---|
| 0 / +00 | QueryInterface, `0090DE02` |
| 1 / +04 | AddRef, `0094C4B1`: increment +8 modulo 2^32; return new count |
| 2 / +08 | Release, `0090E242`: dispatch slot 10 |
| 3 / +0C | InitializeCOM(com,path), `0090DE57` |
| 4 / +10 | Start(com): Tools/game no-op true `0091559A`; ResourceLib `00664B00`; ResMan `0097530F` |
| 5 / +14 | EnumClassObjects(callback,context), `0090E02B` |
| 6 / +18 | GetClassObject(classID,iid,out), `0090DFAC` |
| 7 / +1C | CanUnloadNow, `0090DEA2` |
| 8 / +20 | No-argument success hook, `00648F00` |
| 9 / +24 | Return current +8 reference count, `009C32AC` |
| 10 / +28 | RemoveRef, `0090DE43`: decrement only when nonzero; return count; no deletion |
| 11 / +2C | Return stored heap pointer +28, `009CDC23` |
| 12 / +30 | Return borrowed COM pointer +24, `009D858F` |
| 13 / +34 | Concrete director ID getter (table below) |
| 14 / +38 | GetLibraryPath(outString), `0090DECD`: assign stored bytes/length through string slot 3 and return true |
| 15 / +3C | GetHeapAllocatedSize, `0090DE52` → `00949B47`: return zero |
| 16 / +40 | Deleting destructor; see §2.4 |

| Owner | Primary / hooks tables | Constructor | ID getter → ID | Deleting destructor |
|---|---|---|---|---|
| Tools | `00AD4F60` / `00AD4F34` | `0094ABC9` | `0094AC21` → `1CEC20BE` | `0094AC27` |
| Game | `00A87E70` / `00A87E44` | `00460090` | `00460F40` → `63CC0AF8` | `00989DA0` |
| ResourceLib | `00AAD3C8` / `00AAD39C` | `00664920` | `00664AF0` → `84FE3294` | `00989DA0` |
| ResMan | `00AD8BA0` / `00AD8B74` | `009751DF` | `00975309` → `C3CAEC3B` | `0090E117` |

QI accepts IID 1 and `A21EE941` as the primary identity and IID `03FA40BF`
as object +4. Each success writes the required output pointer, calls primary
AddRef, and returns true in AL. Unsupported IID returns false without changing
the output. The secondary face's slots 0–2 adjust `this` by -4 and share the
same QI/count (`00460F50`, `00460F60`, `00460F70`). None accepts a message-target
IID. A zero count does not destroy a director.

The hooks table has **11 slots total**, not 12: three identity slots and eight
no-argument Boolean hooks, offsets +0C through +28. At +2C the adjacent primary
table begins. Tools, game and ResMan have only no-op true bodies (`00949ADE`)
in those eight positions. ResourceLib has true body `00648F00` except slot 4
(+10), which is `00664540`, specified in §5.3. No hook at +2C exists on these
faces. Table extents are directly established by the table addresses above.

### 2.2 Initialization and adding directors

InitializeCOM(null,path) returns false and preserves prior fields. Otherwise
it stores the borrowed COM pointer, obtains the heap through COM slot 4, copies
the path, initializes each current child in vector order with the same inputs,
ignores child Boolean results, and returns true. It neither AddRefs COM nor
guards against repeated initialization (`0090DE57–0090DEA1`).

AddDirector(child), nonvirtual body `0090E0D4`, first calls child.InitializeCOM
with the receiving director's COM pointer and path. It then adds each of that
child's own children recursively to the receiver, before appending the child
pointer itself. Thus the receiving vector is flattened in descendant-before-parent
order as well as retaining the child director. It does not remove duplicates,
retain child pointers, or use a failed InitializeCOM result to cancel insertion.
Null child pointers and cyclic child graphs are outside this routine's valid
input contract; no guard is present. Container allocation failure is not a
native false-return channel. Construction/destruction of the containing graph
must keep borrowed child pointers alive.

### 2.3 Class registration and dispatch

AddCls `0090E133` inserts a unique class-ID entry with no-argument factory and
kind 1. Its overload `0090E160` inserts a callback of kind 2 accepting `(iid,out)`.
Duplicate IDs do not replace an existing entry. All constructor registrations
listed in §§3,5,6 and Appendix A use kind 1. Their insertion results are ignored.

GetClassObject asks children in vector order, stopping at the first true AL;
it then tries its own registration. An unknown class/kind returns false without
clearing `out`. A kind-1 factory returns an object pointer, whose QI receives
the requested IID and the caller's output. Success keeps the QI reference and
returns true; failure calls that object's AddRef and Release, then returns false.
For kind 2, the registered callback supplies the result directly.

There is **no null check** between a kind-1 factory call and object QI
(`0090DFFC–0090E00B`). Although many allocation wrappers can return null, this
dispatcher does not turn that into false. A failed service creation in §§5–6
means a completed COM request with null output, not a claim that every native
out-of-memory path returns normally. *[choice]* A clean-room host may report
an allocation/boundary fault instead of dereferencing null; do not present that
as an observed native Boolean result.

EnumClassObjects visits every child in vector order, then each local hash entry.
Each local callback receives `(classID, 0, context)`; callback return values are
ignored (`0090E039–0090E079`). Construction order is not enumeration order.
Cross-child duplicate IDs are enumerated more than once; lookup still stops at
the first successful child. Exact local hash iteration is open (ADIR1).

### 2.4 Unloading and destruction

CanUnloadNow returns true immediately when the count at +8 is nonzero.
At zero it asks children in order and returns false at the first child false,
otherwise true (`0090DEA2`). This counterintuitive rule applies to all four
receivers; it is not a conventional outstanding-reference veto.

Base destruction `0090E0A5` destroys the registration table, frees child-vector
storage and destroys the path string. It does not Release borrowed children or
COM, stop services, or send shutdown messages. Each deleting destructor in §2.1
also frees the allocation through `GlobalHeapFree` iff its flags' bit 0 is set,
and returns the original pointer. Static getter storage must not be freed by a
caller choosing flag 1. Normal Release never enters this deleting path.

## 3. Tools director *[compat]*

The complete director contract and service method inventories are
[tools-com-director §§2–5](tools-com-director.md). In bootstrap terms, getter
`0094AC43` returns borrowed static object `00B634D0`; construction registers
only class `3CEC2B47` → factory `0094AB94` and class `9CEC2B58` → `0094A4A6`.
Its Start and eight hooks return true without creating either object,
registering services, subscribing messages or drawing. The audio director's
use of this getter belongs to [audio's director chain](audio.md).

The registered decoder IO, stream ownership, compressed-header validation,
PCM delivery and partial failures are service operations; do not run them as
part of Start. The linked spec's remaining numerical synthesis questions are
not closed by this bootstrap contract.

## 4. Game-data registry *[compat]*

Use [game-data-registry §1](game-data-registry.md) for the full 29-slot inventory
at `00A8CBF8`, including its compiled overload order. Its constructor `004748F0`
is called directly by application AddSystemServices (`004507AE`); the game
director's registrations do not establish a registry factory.

The service face at allocation +4 has service ID `EBE0E860`, priority -1000000
(`FFF0BDC0`), and initially running=false. Registry IID `E898ED03` gives the +0
face; system-service IID `287FB697` gives +4; IID 1 gives the +8 refcount face.
This differs from the director QI failure policy: registry QI clears its required
output before testing IDs. Its own count starts at zero and its Release can
destroy it. Do not apply director lifetime policy to this service.

Initial state is empty node/leaf indexes and empty typed/message target lists,
plus a node named `__root__`, null parent and flags zero, indexed by its own
retained unknown-object key. Service init (`00473A80`, service slot 8) registers
the service face through framework slot 9 and returns true. Shutdown
(`004746A0`, service slot 9) unregisters through framework slot 10, broadcasts
type 1 with null entry, clears the indexes, root children and target lists, and
returns true. It preserves the root record but does not reindex it.

Primitive exchanges/calls, mutation notifications, replay, reference effects and
failure rules are [game-data-registry §§3–5](game-data-registry.md). This owner
dispatches callbacks and notification messages, not raster drawing or byte-stream
IO. The names/values stored by application publishers do not become bootstrap
director state. See that spec's vectors for null/invalid entries, duplicate keys,
subscription order and shutdown behavior.

## 5. ResourceLib director *[compat]*

### 5.1 Construction and factories

Getter `00664B60` returns static object `00B468A0` without AddRef. It tests bit 0
of `00B468EC`, sets it before constructor `00664920`, and registers exit callback
`00A7C6D0`. Subsequent calls skip construction. There is no synchronization or
wait for a recursive first call; exit-registration return is ignored.
Exception/unwind retry policy for this getter is open (ADIR2).

There are 25 kind-1 registrations. The 20 editor/catalog/helper classes and their
factory addresses are already tabulated in
[resource-exemplar-editor §8](resource-exemplar-editor.md), in addition to:

| Class ID | Factory | Returned construction boundary |
|---|---|---|
| `453429B3` | `00664240` | Exemplar resource factory, allocation 28 hex bytes, constructor `0067E360` |
| `053429C8` | `00664260` | Cohort resource factory, allocation 28 hex bytes, constructor `0067E3A0` |
| `252EA0DF` | `00664220` | Exemplar cache: allocation 4C hex bytes, constructor `00668BA0`, returned face at allocation +4 |
| `69B6F01C` | `00664510` | 10-hex-byte object, constructor `00808900`; service semantics remain open (ADIR3) |
| `5CF9459A` | `00681CA0` | nSCRes file parser; [sc-resources §4](sc-resources.md) |

The constructor registers the three first rows before the editor classes;
`69B6F01C` and the file parser are its final two entries
(`00664ABC–00664ADA`; parser ID getter `00680890`). Registration does not open
an editor, parse a file or decode an exemplar. Factory wrappers return null on
allocator-null and do not themselves AddRef; §2.3 covers dispatcher consequences.
Some return adjusted interface pointers: the cache returns +4 and the happiness
curve editor factory `006648F0` returns +D8, not its allocation base.

### 5.2 Start: cache and hook registration

Start `00664B00` obtains COM through its own primary slot 12, **not** the supplied
Start argument. It requests class `252EA0DF` with service IID `287FB697`, with
output initially null. It ignores COM's Boolean result and tests the output
pointer. If nonnull it passes that face to framework slot 3 (+0C), then Releases
the local reference. Whether this succeeds or not, it passes its hooks face
(director +4) to framework slot 7 (+1C). It ignores both framework return values
and returns true in AL (`00664B24–00664B5D`).

Start has no local started flag. Repeating it repeats both requests; whether a
framework deduplicates an insertion is the framework's contract. A live stored
COM and framework are required; no null guard protects their virtual calls.

### 5.3 Hooks slot 4: resource-type binding

Callback `00664540` requires no explicit inputs. It queries the global framework,
if present, for service `056B906E` with manager IID `656B8EFC`, initially null.
It tests the returned pointer, not the lookup Boolean. With a manager it invokes
manager slot 19 (+4C) twice, in order:

| Factory class ID | Resource type | Supplied factory |
|---|---|---|
| `453429B3` | `6534284A` | null (manager creates it) |
| `053429C8` | `05342861` | null (manager creates it) |

It ignores both registration results, Releases the acquired manager and returns
true. Missing framework or missing manager produces false, no registrations and
no retained reference (`00664548–006645A9`). A failure of the first registration
does not suppress the second and does not cause rollback. Manager registration
can append duplicates on repeat: [persistence-manager §4](persistence-manager.md).

This callback is dispatched through the framework hooks list at `0087A30B`:
startup calls that dispatcher at `0087B02A`, after service phase `0087B01E`,
before the following service phase at `0087B046`. The dispatcher ignores each
hook's AL (`0087A30B–0087A31C`); missing manager therefore does not itself abort
this framework phase. The director's other seven lifecycle hooks are true no-ops.

### 5.4 Drawing and IO boundary

The director exposes class lookup and lifecycle hooks, not a draw surface or file
handle interface. Drawing occurs on the constructed view/window faces under
[resource-exemplar-editor](resource-exemplar-editor.md) and
[resource-catalog](resource-catalog.md); file parsing occurs through the parser
face in [sc-resources §4](sc-resources.md). The returned face can differ from the
allocation address; clients must use QI and the returned vtable rather than
casting the director into an editor or stream. Resource-type binding above is
the IO dispatch contribution of this owner; file-format parsing and persistence
error handling belong to the created factory/manager contracts.

## 6. ResMan director *[compat]*

### 6.1 Construction, method inventory and registrations

The shared slots in §2 are its entire director interface. Getter `00975363`
returns borrowed static object `00B63690`; bit 0 of `00B636DC` is set before
construction, and exit callback `00A7F6EE` is registered once. No synchronization,
wait, AddRef or exit-registration result check occurs. Exception retry remains
ADIR2. Constructor `009751DF` registers these 16 kind-1 entries in order:

| Class ID | Factory |
|---|---|
| `0A32C0DA` | `00975062` |
| `090F8619` | `0097507B` (multi-packed-file segment) |
| `086D4A89` | `00975091` |
| `A079CE1B` | `009750B5` |
| `656B8F2D` | `009750CE` |
| `00436EB4` | `00975197` |
| `656B8F27` | `009750E4` (persistence resource manager) |
| `02AE197F` | `009750FD` |
| `42E411C2` | `009751BB` |
| `BB194C78` | `00975113` |
| `5B101E6B` | `00975129` |
| `BB100367` | `0097513F` |
| `3B0AB2B4` | `00975155` |
| `035990A3` | `0097516B` |
| `0A63DF8C` | `00975181` |
| `E953DA68` | `009851C7` (ID getter `00984F9A`) |

IDs identify available classes, not service IDs or resource types. No file is
opened by the director constructor. Segment/record IO is a created-object
operation; [persistence-manager](persistence-manager.md) governs manager dispatch.
Remaining factory-owned interfaces are explicitly bounded by ADIR3.

### 6.2 Start and failure paths

Start `0097530F` uses its **supplied COM argument**, rather than primary slot 12.
It requests class `656B8F27` as IID `287FB697`, output initially null. A nonnull
output is passed first to global framework slot 3 (+0C), then slot 11 (+2C),
then Released. The second registration places the service on the update list;
it is not a call to that service's own slot 11. See
[persistence-manager §9.1](persistence-manager.md).

Creation Boolean and both framework results are ignored. Null output skips both
registrations and Release; either path returns true (`0097532D–00975362`).
A nonnull output is used even if creation returned false. There is no rollback,
started guard, or null-framework check. Repeating Start repeats creation and
registration. All eight hooks are no-op true; no director message subscription,
draw callback, record read or record write is installed by those hooks.

The manager allocation wrapper `009750E4` allocates 9C hex bytes, returns null
on allocator-null, otherwise transfers to constructor `009744B2`. It does not
AddRef. The service face returned by QI is the reference retained until the
Start-local Release, distinct from the manager's public IO face.

## 7. Game COM director *[compat]*

### 7.1 Construction and subordinate director order

Getter `00460F80` returns static object `00B214F8`, without AddRef. It sets bit 0
of `00B21544` before first construction and registers exit callback `00A7AE10`.
Repeated calls return the same storage without re-registering. There is no lock
or recursive-call wait; exception retry is ADIR2. Start and all eight hooks are
true no-ops: its constructor performs the bootstrap wiring.

Constructor `00460090` obtains the **global** root director through `0087B212`.
If that pointer is null it skips the following whole group, but still registers
its own classes. Otherwise it captures global COM once (`0090DDF1`), adds these
14 directors to that root using §2.2, and immediately invokes Start(COM) only
where marked. None is added to the game director's own vector by these calls.

| Order | Getter / director | Immediate Start? |
|---|---|---|
| 1 | `00466400`, scripted window | yes |
| 2 | `00466440`, SC4 GDrivers | yes |
| 3 | `00896012`, fonts | no |
| 4 | `00825C40`, GZ drawing | yes |
| 5 | `009898E0`, graphics director getter/body | no |
| 6 | `00975363`, ResMan | no |
| 7 | `00975037`, Windows resource director | yes |
| 8 | `00998E48`, GZ window director | no |
| 9 | `0090ADE3`, image/Gimex director | yes |
| 10 | `00664B60`, ResourceLib | yes |
| 11 | `009444CF`, GZ script director | yes |
| 12 | `0094B053`, UI script director | yes |
| 13 | `00514C30`, automata | yes |
| 14 | `00880180`, SC Lua director | yes |

Evidence: `004600A6–004601E3`. AddDirector and immediate Start results are ignored.
There is no conditional retry or removal on false, and no per-getter null check.
The absence of immediate ResMan Start in this constructor must not be “fixed”
by inserting one; its normal COM/framework start phase is a separate operation.
The application subsequently adds its own services under
[application §4.2](application.md), including the direct registry construction.

### 7.2 Own class dispatch and drawing/IO

There are **190** kind-1 class registrations at `004601E4–00460F30`;
Appendix A records every class/factory pair in construction order. Getter-based
IDs there were verified as immediate 32-bit returns in native instructions;
the eleven `49B9E6xx` entries use literal IDs. The constructor does not instantiate
these objects or invoke their drawing/IO methods. In particular class IDs are
not IIDs: a request still passes the desired IID to the new object's QI.

For message/queue creation use [framework-gz-services](framework-gz-services.md),
for Lua dispatch [lua-host](lua-host.md), and for rendering/drawing the linked
renderer, model and drawing contracts. This director has no message-target face,
message-ID switch, raster callback or stream virtual slots of its own. Notifications
observed on constructed game services belong to those receivers, not to Start.
The class table is a discoverability contract, not a replacement specification
for all 190 object implementations (ADIR3).

## 8. Conformance vectors

These are independent, literal test fixtures for core/adapter implementations;
they are not reports of native game execution. Use instrumented COM, framework,
factory and child-director interfaces to observe calls and references. Do not
assert upper EAX bits for Boolean methods; the native decision is in AL.

| ID | Input / setup | Required result / observation |
|---|---|---|
| AD-V1 | Fresh director, count 0; QI `03FA40BF`, then QI 1 on that face | Success, identities primary+4 then primary; one shared count becomes 2 |
| AD-V2 | Director out initially sentinel; QI `FFFFFFFF` | False; sentinel unchanged; count unchanged |
| AD-V3 | Director Release at count 0; then AddRef, Release | Counts 0,1,0; no destructor |
| AD-V4 | Initialized with COM A/path `old`; InitializeCOM(null,`new`) | False; old COM, heap, path and child state preserved |
| AD-V5 | Two children return false from InitializeCOM; parent initialized with COM A/path `x` | Both called in vector order; parent stores A, A's heap, `x`; true |
| AD-V6 | Add child C with child D to root R | Initialize C with R inputs, recursively add D, append C; vector suffix D,C; no AddRef on D/C |
| AD-V7 | Two children can create class 1234; local registration also exists | First child's true stops lookup; second/local factory uncalled |
| AD-V8 | Unknown class; out sentinel; children all return false without output changes | False; sentinel unchanged |
| AD-V9 | Kind-1 factory object's QI rejects IID | False; one factory call, QI, AddRef, Release; no forced output clearing |
| AD-V10 | Count 1, child CanUnloadNow false; repeat with count 0 | First true without child call; second false after child call |
| AD-V11 | Enum callback returns false for first entry | Enumeration continues; local callback's second argument always 0; child phase precedes local phase |
| AD-V12 | Invoke each valid Tools/game/ResMan hooks slot 3–10 | True; no external calls; slot 11 is outside hooks table |
| AD-V13 | ResourceLib stored COM A, Start argument B; A returns null, Boolean true | Only A receives `(252EA0DF,287FB697)`; framework receives hooks at slot 7; no service registration/Release; true |
| AD-V14 | ResourceLib COM returns service S, Boolean false; framework service registration returns false | Framework slot 3(S), S.Release, framework slot 7(hooks), in order; true |
| AD-V15 | ResourceLib hooks slot 4, global framework absent or manager output null | False; zero factory registrations; no dangling reference |
| AD-V16 | ResourceLib hook gets manager M despite false lookup Boolean; first RegisterFactory returns false | Calls `(453429B3,6534284A,null)`, then `(053429C8,05342861,null)`, M.Release; true; no rollback |
| AD-V17 | Shipped framework dispatch of ResourceLib slot 4 returns false | Dispatcher continues subsequent hooks and returns true when not disabled; false hook does not itself abort phase |
| AD-V18 | ResMan stored COM A; Start argument B returns service S with false Boolean | B receives `(656B8F27,287FB697)`; framework slots 3(S),11(S), S.Release; true |
| AD-V19 | ResMan supplied COM returns null output | True; no framework registrations or Release |
| AD-V20 | Game constructor global root absent | No subordinate getter/Start calls; all 190 own registrations still attempted |
| AD-V21 | Game constructor root present; child Start reports false | Continue 14-director sequence; only ten immediate Starts in §7.1; no rollback |
| AD-V22 | Construct ResourceLib / ResMan; enumerate local registrations with no children | Class sets have 25 / 16 entries; matching class returns listed factory's QI face, not a director face |
| AD-V23 | Registry QI unknown with sentinel output | False and output null, unlike AD-V2; registry vectors in linked spec cover remaining entry methods |
| AD-V24 | Registry shutdown after root and children exist | Framework unregister, type-1 null-entry notification, indexes/subscriptions empty; root pointer retained but IsValidEntry(root)=false |
| AD-V25 | Any director GetLibraryPath after storing `abc`, then GetHeapAllocatedSize | Output bytes `abc`; true; size 0 irrespective of registrations |
| AD-V26 | Two calls to each singleton getter after successful first construction | Same borrowed pointer; no second constructor/exit registration; no getter AddRef |
| AD-V27 | Destructor flags 0 versus 1 on heap-backed director fixtures | Both destroy table/vector/path; only flags 1 frees allocation; no child Release |

Fault fixtures should distinguish an unknown class's ordinary false from the
kind-1 allocator-null boundary in §2.3. No vector promises null-dereference
recovery, thread-safe getter construction or exception rollback.

## Appendix A. Game director class inventory *[compat]*

Each pair is `class ID → factory address`, ordered left to right and top to
bottom. Ordinals are documentation indices, not ABI values or enumeration order.
All values are hexadecimal. Evidence is constructor `00460090`, registration
calls `004601F1–00460F30`, and the immediate-return class-ID getters it calls.

| Ordinals | Class → factory pairs |
|---|---|
| 1–4 | `252294E9 → 0092EBB7`; `452294DF → 0092EA44`; `46DF45E7 → 009135E8`; `CBD101A9 → 008B10D0` |
| 5–8 | `EBA2BE1A → 008AFD80`; `6BA6CD0C → 008B3780`; `ABD63ED3 → 008AFFB0`; `6BD941EB → 008B0040` |
| 9–12 | `EBD9423C → 008B00D0`; `E9C622D8 → 007CB070`; `E9C6262A → 00800320`; `C9C628EC → 007CCC00` |
| 13–16 | `A9C73857 → 00448FF0`; `A9C741B3 → 004929B0`; `3AB50E2A → 008005B0`; `FAD0F0B6 → 00804050` |
| 17–20 | `8990C12D → 005D52E0`; `A83479EA → 005ECE10`; `496678FE → 0053D130`; `0B8DA7DB → 0073E950` |
| 21–24 | `E98F9525 → 0074D580`; `C9B84E10 → 00757470`; `A7E7F929 → 00445C80`; `8990C372 → 0045A560` |
| 25–28 | `6A037C24 → 004AD9A0`; `0990C38A → 00466E40`; `C990BD46 → 00613750`; `4990BD93 → 0046A7B0` |
| 29–32 | `A990BD10 → 004A28E0`; `298F9B2D → 004DD3E0`; `299062C0 → 0048A520`; `89C48F47 → 006F3600` |
| 33–36 | `89C49986 → 006F1500`; `E9A6CD3B → 0060F4F0`; `0990C138 → 006B7870`; `C9BD5D4A → 006C2FC0` |
| 37–40 | `4A232DA8 → 006C0EB0`; `498F9B01 → 00734AF0`; `A990BFE0 → 006C8DF0`; `2990C142 → 0070CAA0` |
| 41–44 | `89EFA536 → 00476BB0`; `6A0F82B2 → 004B42E0`; `EA09D4AE → 006B98E0`; `AA2049A1 → 0068B520` |
| 45–48 | `EA230A28 → 007019F0`; `AA3FE6BC → 006D5AE0`; `EAD4BA36 → 0045D420`; `A956AD14 → 0059F860` |
| 49–52 | `49822F75 → 005919B0`; `2A5118AD → 005A5110`; `EA5118B5 → 005A3BD0`; `0A1D8285 → 007D1030` |
| 53–56 | `9AC0E23C → 0080A680`; `9AC0E23D → 00800E30`; `49B9E601 → 0045F730`; `49B9E602 → 0045F730` |
| 57–60 | `49B9E603 → 0045F780`; `49B9E604 → 0045FFF0`; `49B9E605 → 0045F7D0`; `49B9E606 → 00460040` |
| 61–64 | `49B9E607 → 0045F820`; `49B9E608 → 0045F870`; `49B9E609 → 0045F8C0`; `49B9E60A → 0045F910` |
| 65–68 | `49B9E60B → 0045F960`; `098F964D → 004A18A0`; `A9BC9AB6 → 005E9330`; `A9BD882D → 00458F40` |
| 69–72 | `49C05B9F → 006489B0`; `CA16374F → 00653560`; `EA0A5ACF → 00648440`; `CA15ED6B → 00652CB0` |
| 73–76 | `29D25F1D → 004715E0`; `C9C05C5D → 0064AB70`; `09C05C6A → 0064E340`; `C9C05C6E → 0061C480` |
| 77–80 | `49C1A034 → 0061E820`; `49CC1BCD → 00605450`; `8A4BD52B → 006477F0`; `49B6D69B → 005F2060` |
| 81–84 | `2977AA47 → 005EEF50`; `2977AA48 → 005EF180`; `2977AA49 → 005F0560`; `2A4017A0 → 005F0490` |
| 85–88 | `C9EFB016 → 005EFC80`; `A9C05C85 → 00472C30`; `A97F909E → 005D0BC0`; `49C05C8F → 005D64C0` |
| 89–92 | `C97F987C → 005E0820`; `49C05C9F → 005E3330`; `8A22BEBE → 004776B0`; `AB72FBB3 → 005F5CE0` |
| 93–96 | `8B79C707 → 005F84D0`; `A9C74DC9 → 005EA130`; `C9C75EAE → 005EA160`; `E9C75ED1 → 005EA190` |
| 97–100 | `E9C75EE8 → 005EA330`; `CA0E3852 → 007A0420`; `AA25CB90 → 005EB970`; `8A7A1C2C → 005EA240` |
| 101–104 | `EAC3A3D2 → 005EA1C0`; `AAC3C01D → 005EA200`; `0BCEB6C2 → 005EBBD0`; `6A19BDD8 → 00658370` |
| 105–108 | `4A19BDE3 → 00658390`; `4B8EA424 → 0060AAA0`; `2990C1E5 → 00704340`; `2990C1BC → 00729310` |
| 109–112 | `C990C034 → 006B2940`; `0990BDD8 → 0068D6E0`; `E990BE01 → 00694970`; `8990BE7A → 0069A7A0` |
| 113–116 | `8990C080 → 0069EF70`; `8990C09A → 006A05C0`; `2990BFF2 → 006A6F30`; `E990BFFC → 006AB000` |
| 117–120 | `AA313C9F → 006AB7A0`; `2990C05A → 006ADB80`; `E990C045 → 006AEAD0`; `A990C022 → 006B8AD0` |
| 121–124 | `E990C18E → 006BDEF0`; `C990C0A4 → 006D38D0`; `0A134F24 → 006D4B90`; `2A134F42 → 006CF940` |
| 125–128 | `A990C08E → 0080E890`; `0990C075 → 006DC9A0`; `6990C04F → 006E2720`; `8990C065 → 006E5F00` |
| 129–132 | `0990C005 → 006F00D0`; `4990C013 → 00700E60`; `6990C1AA → 00721580`; `0A413481 → 0069E800` |
| 133–136 | `29E37DEC → 0042F6B0`; `0C0B06E8 → 006E0030`; `4C0AE114 → 006ACEA0`; `61F6ABF5 → 00578B70` |
| 137–140 | `69C1718E → 00579AE0`; `69C1718F → 0057BB60`; `0A086BD3 → 00575670`; `AA086D86 → 005763F0` |
| 141–144 | `4A132FDF → 0057C0E0`; `0A133012 → 0057D000`; `A9B5AB9F → 0057FE00`; `29B5AB89 → 0057FF00` |
| 145–148 | `4A38206C → 0057DAA0`; `4A38206D → 0057DFA0`; `6A7B3E9B → 00577200`; `EBFB0402 → 0057EDF0` |
| 149–152 | `8A09F5F4 → 00408A50`; `EAC937B4 → 004100D0`; `4A0DD616 → 0041B220`; `6A26D049 → 0040D270` |
| 153–156 | `AA371C32 → 0043EBD0`; `6A9335DE → 004250C0`; `6A3848DB → 00420E60`; `6B70EFAD → 0042FB10` |
| 157–160 | `EA19E825 → 00415B90`; `CA2C2A2F → 00420DD0`; `4A3AD3E1 → 0041C170`; `AA5292D7 → 00413980` |
| 161–164 | `2A5F877D → 004121A0`; `6A5F8755 → 00413D40`; `CA09F543 → 00410DD0`; `4BE372CD → 004136B0` |
| 165–168 | `8BE3753B → 00423DD0`; `AA3836FB → 0043F760`; `4A1DBBBF → 0042A9F0`; `CBC14674 → 00434620` |
| 169–172 | `0BF606A5 → 0043DFA0`; `8BD4FC74 → 0080CDC0`; `A2BF1DDC → 0080BD70`; `A2BF1E43 → 0080BDC0` |
| 173–176 | `00D0723D → 0080BE10`; `40D07236 → 0080BF30`; `815B4CEF → 0080C0A0`; `22F6E80C → 0080C2C0` |
| 177–180 | `A0D07129 → 0080C360`; `62BF1DAA → 0080C540`; `62BF1DB9 → 0080C600`; `E0D0722E → 0080C660` |
| 181–184 | `22F6E81B → 0080C6B0`; `82B9999B → 0080C700`; `E0D07233 → 0080C850`; `A2BF1DE5 → 0080C8E0` |
| 185–188 | `C2F6E81F → 0080EA30`; `62F6E7CF → 0080EB30`; `42BF1E18 → 0080ED10`; `C2BF1E04 → 0080EDB0` |
| 189–190 | `02BF1DFA → 0080EE00`; `C2BF1DC5 → 0080EE60` |

## 9. Open boundaries

[Questions ADIR1–ADIR3](questions/app-directors.md) track local registration-hash
iteration, the three newly inspected getters' exception retry behavior, and
uncontracted factory-owned interfaces. The known bootstrap and dispatch rules
above do not depend on assigning semantic names to unnamed registered classes.
Existing registry/decoder internals retain the questions in their linked specs.
