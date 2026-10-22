# Spec: graphics-driver COM director (`cSC4GDriversCOMDirector`)

Status: **implementation spec, partial shared-method audit**. Clean-room behavior contract for SimCity 4
Deluxe GOG 1.1.641. Prose, constants and address citations are the implementer-facing output; original
program material must not be consulted by implementers.

## 1. Scope and exact attribution

This owner publishes factories through the COM director protocol. It is separate from the rendering
operations in [renderer-driver.md](renderer-driver.md). No new coverage of `nSGLDX7`, `nRZSimGL`,
`cGZBufferSimGLCache`, or RD3–RD5 is claimed here.

The export's exact owner spelling is **`cSC4GDriversCOMDirector`**, without `8068B`. Its directly named
inventory is **two bodies, 85 bytes**. A folded deleting destructor also names this class among eight
aliases, but its 30 bytes are shared and must not be counted as another exclusive director body.
The task's `8068B` suffix is not evidence for an 8,068-byte body inventory. The singleton accessor,
registered factories and inherited bodies below are supporting evidence, separately accounted for.
No native object layout is prescribed.

| Attributed body | Address | Export size | Checklist / contract |
|---|---|---:|---|
| Constructor | `0x00466250` | 79 | specified, §2: base construction and three registrations |
| `GetDirectorID` | `0x004662A0` | 6 | specified, §2: `0x64B2A903` |
| Folded deleting destructor, alias of `icf::cGZDrawCOMDirector::~cGZDrawCOMDirector` | `0x00989DA0` | 30 shared | shared destruction audit pending, §6 |

The primary vtable at `0x00A87EE8` has 17 slots. Only slot 13 is a director-specific override;
the constructor also installs a secondary interface table at `0x00A87EB8`.
The complete slot checklist and secondary dispatch audit will be recorded in §6. The existence of
this table does not imply additional driver APIs, device enumeration, or a director-specific renderer
start/stop protocol.

Evidence: export function inventory, `decomp/index.tsv`, `decomp/cSC4GDriversCOMDirector.c`, folded
alias inventory at `0x00989DA0`, and instructions at `0x00466250–0x0046629E` / `0x004662A0–0x004662A5`.

## 2. Construction, identity and lazy access

The logical state comprises a reference counter, a second unload-related counter, a library-path
string, service and framework handles, an ordered child-director collection, and a class registry.
The base constructor sets both counters to zero, constructs an empty path and registry, clears the
service/framework handles, and creates an empty child collection (`0x0090E18D`).

The derived constructor then installs this director's interface dispatch and makes these three
registrations, in this order. All use the **no-argument object factory** registration kind (tag 1),
not the alternate factory kind that receives an interface ID and output argument.

| Registration order | Class ID (32 bits) | Factory entry | Distinguishing identity |
|---:|---|---|---|
| 1 | `0xC4554841` | `0x0087E1D0` | first driver factory; the getter at `0x0087E0A0` returns this ID, despite its misleading DX7 export name |
| 2 | `0x7ACA35C6` | `0x008DCD30` | second driver factory; getter `0x008DA490` |
| 3 | `0xBADB6906` | `0x00886700` | DX7 driver factory; getter `0x00886E00` |

Registration stores a class ID, a factory and kind 1 with unique-key insertion (`0x0090E133`). It does
not invoke a factory. The derived constructor does not inspect registration results, undo earlier
registrations, initialize a device, or query a service. On ordinary completion it returns the director.
Allocation exceptions are not converted into a Boolean failure by these bodies. Registry insertion
and iteration order are audited separately in §5; registration order is not a promise of enumeration
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
3. Assign the supplied path through the director's string operation (string slot 13).
4. Call each child director's `InitializeCOM` in child order with the **same service and original path
   arguments**. Disregard each child's Boolean result.
5. Return true.

The path is not the service pointer: instruction argument tracing at `0x0090DE6E` and `0x0090DE82`
disambiguates the unreliable pseudocode. Repeated successful calls repeat the service query, overwrite
the remembered handles and path, and revisit children. A later null-service call returns false and
retains the last successful state. This director starts with no children and adds none in its derived
constructor. No factory is invoked by `InitializeCOM`.

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
instances, hardware adapters, or display modes. The exact local registry order remains under audit.
Evidence: child calls `0x0090E043`; callback arguments/call `0x0090E05E–0x0090E068` and unconditional
iterator advance `0x0090E071`.

## 6. Shared dispatch and destruction checklist

Pending: complete all primary and secondary interface targets, unload/reference quirks, inherited
no-op/constant slots, registry iteration order and singleton exit destruction. This does not reopen
driver methods or RD3–RD5.

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
| GD11 | Enumerate with callback returning false on its first call | All local classes still reported once, each with second argument 0 and original context; zero factory calls; exact order to be completed in §5 |
| GD12 | Two successful requests for the same local class | Two factory invocations; no director cache or initialization prerequisite |
