# Tools COM director (Windows 1.1.641)

## 0. Scope and evidence

This specifies `cGZToolsCOMDirector`, separately from `cSC4App`, the automata director,
and other COM directors. The brief's `31293B` is a byte count, not part of the class name.
All behavioral rules below come from the Windows executable and exported instructions;
exported pseudocode was used only as a navigation aid. Addresses are executable virtual
addresses. No implementation or test output supplies a contract.

The attribution audit at the lane's base (`a1804eeb`) assigns **70 bodies / 31,293 bytes**
to this owner: two by medium-confidence names, one by neighbor, and 67 by reachability.
Attribution is an inventory boundary, not proof of class membership. In particular,
many bodies are decoder helpers reached through the registered objects. This document
does **not** yet justify complete-owner coverage; see §1 and questions TD1–TD3.

## 1. Complete attributed-body checklist

Sizes are exported body-byte sums, not address-span estimates. `D` means director
contract in §2–§4; `S` means registered-service boundary with remaining details in
§5/questions; `U` means the body remains uncontracted (TD1). All rows other than the
constructor/getter and `00A66A31` have attribution rule `reach`; the two named bodies
have `name-medium`, and `00A66A31` has `neighbour`.

| Address | Bytes | Boundary/status |
|---|---:|---|
| `00407000` | 2444 | U; distant reached helper |
| `00420036` | 21 | U; distant reached helper |
| `0094A079` | 107 | U; decoder input helper |
| `0094A0E4` | 455 | U |
| `0094A2AB` | 22 | U |
| `0094A2C1` | 311 | U |
| `0094A3F8` | 58 | U |
| `0094A437` | 32 | S; small-service constructor |
| `0094A457` | 37 | S; small-service QueryInterface |
| `0094A47C` | 28 | S; small-service deleting destructor |
| `0094A498` | 14 | S; small-service destructor |
| `0094A4A6` | 50 | D; class `9CEC2B58` factory |
| `0094A4D8` | 232 | S; large-service constructor |
| `0094A5C0` | 27 | S; large-service Release |
| `0094A5DB` | 8 | S; large-service slot +24 |
| `0094A5E3` | 7 | S; large-service slot +28 |
| `0094A5ED` | 557 | S; large-service slot +38 |
| `0094A81A` | 120 | S; large-service slot +3C |
| `0094A892` | 7 | S; large-service slot +40 |
| `0094A899` | 19 | S; large-service slot +44 |
| `0094A8AC` | 22 | S; large-service slot +48 |
| `0094A8C2` | 8 | S; large-service slot +58 |
| `0094A8CA` | 8 | S; large-service slot +4C |
| `0094A8D2` | 53 | S; large-service slot +50 |
| `0094A907` | 107 | S; large-service slot +20 |
| `0094A972` | 5 | U; tail thunk to `00A66BBE` |
| `0094A977` | 293 | S; large-service slot +14 |
| `0094AA9C` | 28 | S; large-service deleting destructor |
| `0094AAB8` | 75 | S; large-service destructor |
| `0094AB03` | 145 | S; small-service creation/lookup slot +0C |
| `0094AB94` | 53 | D; class `3CEC2B47` factory |
| `0094ABC9` | 88 | D; director constructor (named) |
| `0094AC21` | 6 | D; director ID |
| `0094AC27` | 28 | D; director deleting destructor |
| `0094AC43` | 69 | D; singleton getter (named) |
| `0094AC88` | 545 | S; large-service slot +1C |
| `0094AEAE` | 107 | S; large-service slot +0C |
| `00A667B4` | 38 | U |
| `00A667DA` | 36 | U |
| `00A667FE` | 463 | U |
| `00A66A31` | 46 | U; neighbor attribution only |
| `00A66A5F` | 211 | U |
| `00A66B32` | 49 | U |
| `00A66B80` | 39 | U |
| `00A66BA7` | 23 | U |
| `00A66BBE` | 21 | U |
| `00A66BD3` | 28 | U |
| `00A66BEF` | 189 | U |
| `00A66CAC` | 2078 | U |
| `00A674CA` | 119 | U |
| `00A67541` | 56 | U |
| `00A67579` | 57 | U |
| `00A675B2` | 46 | U |
| `00A675E0` | 3405 | U |
| `00A6832D` | 1414 | U |
| `00A688B3` | 584 | U |
| `00A68AFB` | 286 | U |
| `00A68C19` | 684 | U |
| `00A68EC5` | 319 | U |
| `00A69004` | 97 | U |
| `00A69065` | 2084 | U |
| `00A69889` | 173 | U |
| `00A69936` | 410 | U |
| `00A69AD0` | 457 | U |
| `00A69C99` | 493 | U |
| `00A69E86` | 981 | U |
| `00A6A25B` | 5716 | U |
| `00A6B918` | 3773 | U |
| `00A6C7D5` | 51 | U |
| `00A6C808` | 671 | U |

Shared/inherited bodies used below are additional evidence, not extra inventory rows.
The exit callback `00A7F63C` and base-destructor thunk `0094A432` are examples.

## 2. Construction, singleton, and real registrations

`Get` (`0094AC43`) returns the same static object at `00B634D0`, without AddRef.
On the first call (bit 0 of `00B6351C` clear), it sets that bit **before** constructing
the object, constructs in place, and registers exit callback `00A7F63C`. Later calls
skip both actions. There is no lock, wait, or atomic instruction in this path; concurrent
construction is not promised. Construction failure/reentrancy must not be modeled as
an automatic retry, since the flag has already been set; exception policy is TD3.

The constructor first runs `cRZCOMDllDirector` construction (`0090E18D`), then installs
primary table `00AD4F60` at object +0 and the hooks table `00AD4F34` at +4. Base
construction initializes reference count +8 and +0C to zero; the library-path string
at +10; COM/heap pointers +24/+28 and child-director vector +2C/+30/+34 to zero;
and the registration hash table at +38.

Exactly these two `AddCls` calls follow, in this order (`0094ABE3–0094AC0D`):

| Class ID | Factory | Allocation | Object construction | Returned pointer |
|---|---|---:|---|---|
| `3CEC2B47` | `0094AB94` | `1104` hex bytes | `0094A4D8` | allocation base |
| `9CEC2B58` | `0094A4A6` | `0C` hex bytes | `0094A437` | allocation base |

These are class IDs, not the director ID (`1CEC20BE`, returned by `0094AC21`).
Do not substitute a name inferred from another platform for the observed registrations.
`AddCls` (`0090E133`) inserts a unique hash entry containing class ID, factory pointer,
and kind 1. The constructor neither creates either service nor inspects insertion
results. Both factories take no explicit parameters, use `GlobalHeapAlloc`, call the
constructor only for a nonnull allocation, and return null on allocator-null. They
do not AddRef or translate a null result to a Boolean.

## 3. Interfaces, lifecycle, and native return decisions

Director `QueryInterface` uses base body `0090DE02`. ID 1 and `A21EE941` return the
primary pointer; `03FA40BF` returns object +4. Each success writes through the output
pointer, invokes primary AddRef, and returns true in AL. Any other ID returns false
without writing the output. A valid writable output pointer is required on success.
The +4 face's first three slots subtract four from `this` before dispatching to base
QueryInterface/AddRef/Release. They are not separate reference counts.

Primary AddRef (`0094C4B1`) increments the unsigned 32-bit count at +8 and returns
the new count. Release (`0090E242`) tail-dispatches primary +28 to RemoveRef
(`0090DE43`): decrement if nonzero, otherwise retain zero, and return the count.
It does not delete the singleton at zero.

The +4 hooks face has nine lifecycle slots, +0C through +2C inclusive. **Every one**
points to `00949ADE`, which only returns AL=1. Thus these native hooks have no
tool creation, service registration, message subscription, start/stop side effects,
or failure decision. The table ends at `00AD4F60`; continuing past +2C would read
the adjacent primary table, not more hooks. No director message-handler interface
is accepted by its QueryInterface.

`InitializeCOM(com,path)` (`0090DE57`) returns false for null `com` without changing
the stored fields. Otherwise it stores the borrowed COM pointer at +24, calls COM
slot +10 and stores the returned heap pointer at +28, assigns the path to +10,
then initializes each child director in vector order with the same arguments.
Child Boolean results are ignored; the method returns true. It performs no AddRef
on COM and has no idempotency guard.

`CanUnloadNow` (`0090DEA2`) has a native quirk: a **nonzero** count at +8 returns
true immediately. With count zero, it visits child directors until one returns
false, otherwise returns true. In the freshly constructed tools director the child
vector is empty, so either count yields true. Do not impose a conventional
"nonzero count prevents unloading" rule on this function.

These member calls use ECX for `this`. QueryInterface and InitializeCOM pop 8 bytes;
GetClassObject pops 12; deleting destructors pop 4. Boolean results specified above
are AL results: upper EAX bits are not part of the Boolean contract. Factories and
the singleton getter return pointers in EAX. The getter has no `this` argument.

## 4. Class lookup, enumeration, and destruction

Inherited `GetClassObject(classID,iid,out)` (`0090DFAC`) first asks child directors
in vector order and stops on a true AL. If none succeeds, it looks up its own hash
entry. Unknown class/kind returns false without clearing `out`. A kind-1 entry
(both registrations here) calls its no-argument factory, then invokes the returned
object's QueryInterface with `(iid,out)`. On QI success it returns true and retains
the reference obtained by QI. On failure it calls AddRef then Release on the new
object and returns false. **There is no null test between the factory and the
virtual QI call** (`0090DFFC–0090E00B`); allocator-null is therefore not gracefully
handled by this higher-level path. Direct factory-null and GetClassObject failure
are different contracts. The other registration kind (2) is not installed here.

Inherited `EnumClassObjects(callback,context)` (`0090E02B`) first enumerates children,
then its own hash entries. Each local callback receives `(classID,0,context)` as
three stack arguments, with caller cleanup. Callback return values are ignored;
there is no early cancellation test. Enumeration does not instantiate services.
Construction order is not an enumeration-order guarantee: iteration is through
the hash table. No meaningful return value is established by this body.

The exit callback (`00A7F63C`) invokes the base-destructor thunk on the static object;
it does not call a deleting destructor or free the static storage. Base destruction
(`0090E0A5`) destroys the registration table, frees the child-vector storage if
present, and destroys the path string. It does not Release each child or the COM
pointer in this body. The tools deleting destructor (`0094AC27`) runs the same base
destruction and calls `GlobalHeapFree(this)` only if bit 0 of its 32-bit argument
is set; it returns the original pointer. It is inappropriate for the static getter's
storage, but is the observed virtual slot +40.

## 5. Registered-service boundary

The small service has primary table `00AD4EC0`; its constructor sets reference
count +4 to zero and initializes its list at +8. QueryInterface (`0094A457`)
accepts ID 1 and `FCEFF549`, returns the unchanged base pointer, and AddRefs on
success. Unknown IDs leave the output untouched. Its destructor is `0094A498`;
the deleting wrapper `0094A47C` frees storage only for argument bit 0 set.

The large service has table `00AD4ED8` (ending before `00AD4F34`), a zero initial
reference count at +4, embedded file at +20 and string at +10D4. It is allocated
only by its factory or the small service's creation path, not by director startup.
Its default-format setup (`0094A54E–0094A5AC`) includes fields 1, 2, 44100,
176400 and 16 at +10EC/+10EE/+10F0/+10F4/+10FA. The stored +10F8 value is computed
from the **prior, uninitialized** 16-bit contents at +10FA before 16 is stored;
do not silently replace this instruction order with an assumed constant 4.
No stable external class/interface names are asserted for these services here.
Service slot semantics, decoder behavior, and complete lifetime decisions remain
explicitly open; this is not a complete decoder specification.

## 6. Independent discriminating vectors

These are contract vectors to implement independently, not reports of execution.

| Vector | Setup/action | Required observation |
|---|---|---|
| TD-V1 | Call getter twice from an unconstructed process state | Same pointer; one constructor and one exit registration; no AddRef |
| TD-V2 | Query director IDs 1, A21EE941, 03FA40BF, then an unrelated ID with output sentinel | First two base pointer; third base+4; three count increments; last false and sentinel unchanged |
| TD-V3 | Release a director at count zero, then AddRef twice and Release once | Counts 0, 1, 2, 1; no destruction |
| TD-V4 | Invoke each of nine hook slots with instrumented external services | True AL each time; no external call or state change |
| TD-V5 | InitializeCOM with null, then nonnull COM whose heap callback returns null | First false and no writes; second stores COM and null heap, assigns path, returns true |
| TD-V6 | Enumerate with a callback returning false on its first call | Both registered IDs still visited; middle callback argument zero; no factory allocation |
| TD-V7 | GetClassObject for an unknown class with output sentinel | False; sentinel retained; no allocation |
| TD-V8 | Create class 9CEC2B58 for IID FCEFF549, then for unrelated IID | Success returns object with one reference; unsupported IID destroys the temporary object via AddRef/Release; output unchanged |
| TD-V9 | Directly force either factory allocation to return null | Factory returns null and skips construction; distinguish from GetClassObject's unchecked dereference |
| TD-V10 | CanUnloadNow with positive count and a child that would return false; repeat at zero count | Positive count returns true without child call; zero count calls child and returns false |
| TD-V11 | Destroy heap-created director with flags 0, 2, and 3 in independent cases | All run base destruction; only 3 frees allocation; original pointer returned |
| TD-V12 | Prefill large-service +10FA with 8 or 16 before constructor | +10FA ends at 16 in both; +10F8 differs according to the pre-store calculation; no guaranteed default block alignment |

## 7. Open contracts and coverage boundary

See [tools-com-director questions](questions/tools-com-director.md). The full attributed
checklist is retained so that documenting the four director-specific bodies cannot
accidentally award all 31,293 bytes. Startup/lifecycle hooks are confirmed for the
Windows tables; no other-platform `OnStart`/`PostAppInit` behavior is imported.
