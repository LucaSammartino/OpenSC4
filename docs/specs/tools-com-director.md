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
contract in §2–§4; `S` means registered-service boundary with rules in
§5/questions; `U` means the body remains uncontracted (TD1). All rows other than the
constructor/getter and `00A66A31` have attribution rule `reach`; the two named bodies
have `name-medium`, and `00A66A31` has `neighbour`.

| Address | Bytes | Boundary/status |
|---|---:|---|
| `00407000` | 2444 | U; distant reached helper |
| `00420036` | 21 | A; numeric-table false reference (§1.1) |
| `0094A079` | 107 | S; compressed-input submission (§5.7) |
| `0094A0E4` | 455 | U |
| `0094A2AB` | 22 | S; decoder reset boundary (§5.3) |
| `0094A2C1` | 311 | S; compressed-header parser (§5.6) |
| `0094A3F8` | 58 | S; ID3 prefix length (§5.4) |
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
| `0094A972` | 5 | S; tail thunk to decoder destruction |
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
| `00A667DA` | 36 | S; history clear (§5.9) |
| `00A667FE` | 463 | S; complete internal header parser (§5.9) |
| `00A66A31` | 46 | S; inverted header check (§5.9); neighbor attribution retained |
| `00A66A5F` | 211 | U |
| `00A66B32` | 49 | U |
| `00A66B80` | 39 | S; cursor reset (§5.9) |
| `00A66BA7` | 23 | S; decoder logical cleanup (§5.3) |
| `00A66BBE` | 21 | S; decoder destructor (§5.3) |
| `00A66BD3` | 28 | U |
| `00A66BEF` | 189 | U |
| `00A66CAC` | 2078 | U |
| `00A674CA` | 119 | U |
| `00A67541` | 56 | U |
| `00A67579` | 57 | U |
| `00A675B2` | 46 | U |
| `00A675E0` | 3405 | S; complete layer-3 side-information parser (§5.8) |
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

### 1.1 Attribution discrepancy and large-body evidence

`A` denotes an attribution artifact. The reference to `00420036` originates at
`00AF5218`, which native loads at `00A68D6F` and `00A6935B–00A693B0` treat as
signed 16-bit numeric table entries. Its words 54 and 66, packed together, happen
to equal `00420036`. Executable bytes at `00420030` place that alleged entry
inside the immediate of a compare at `00420034`. Its exported 21-byte body has no
direct callers. Retain the row for the audit; TD4 requests attribution correction,
without changing maps/exports. Excluding it leaves 69 candidates / 31,272 B, not
69 fully specified bodies.

The largest bodies were inspected to establish the remaining boundary:
`00A6A25B` (5716 B) updates a modulo-16 phase at decoder +4A0C and reaches MMX
routine `00407000` (2444 B); `00A6B918` (3773 B) uses x87 float operations, global
scratch at `00BB87A8` and float coefficient tables; `00A675E0` (3405 B) extracts
packed bits through +24A4/+24A8; `00A69065` (2084 B) uses the numeric word tables;
`00A66CAC` (2078 B) initializes float outputs and performs further arithmetic.
The side-information body `00A675E0` now has a complete contract in §5.8.
The other five bodies remain TD1; their complete numerical outputs are not proved.
These observations do not justify a presumed stateless double-precision decoder.

The following additional boundary facts are instruction-verified, but do **not**
close the 5716-byte synthesis body. `00A6A25B` takes decoder ECX and two stack
arguments `(PCM destination, subband input)`, popping eight bytes. It decrements
the byte at +4A0C modulo 16 even when signed channels at +4A40 is nonpositive.
Let the resulting phase be p, parity b=p&1, and q=p+(b XOR 1). For each channel c
from zero to channels-1 it calls `00407000` with two borrowed history destinations:
`this+4+c*480h+2*(((p+b)&15)+b*120h)` and
`this+4+c*480h+(b XOR 1)*240h+2*q`; its third argument is input+c*40h.
Thus each channel consumes 64 bytes of packed input, not 32 binary32 values.
The history used for windowing starts at `this+4+c*480h+(b XOR 1)*240h`.
The call occurs even when the PCM destination is null; null skips only windowing
and output stores. For exactly two channels it writes 32 signed words per channel
at destination+2*c+4*j (j=0..31); otherwise the stride is two bytes, with each
channel's base still advanced by two. There is no channel-limit, pointer, length,
allocation or error check. Inputs and outputs are borrowed, with no ownership
transfer. Output values use signed word products, wrapping dword accumulation,
arithmetic right shift by 13 and signed saturation to 16 bits; the central sample
has a distinct multiply-high path whose complete coefficient contract remains
open. No defined EAX result or local EMMS is provided by this body.
Evidence: phase/setup `00A6A269–00A6A2F1`, output dispatch `00A6A2F9–00A6A310`,
first sample `00A6A32D–00A6A388`, central multiply-high `00A6A83C–00A6A893`,
mono/stereo branches and loop tail `00A6ADB9–00A6B8AC`.

`00407000` has no calls, branches, allocation or input-length check. It reads 64
input bytes and writes 17 words at firstDestination+40h*k (k=0..8) and
firstDestination+20h+40h*k (k=0..7), and 16 words at
secondDestination+20h*k (k=0..15). Only these word locations are stores; gaps
remain history. It returns without EMMS. Its complete butterfly/coefficient
arithmetic remains open: the scattered stores and packed signed-word operations
must not be replaced by an assumed floating transform. Evidence: `00407000–0040798B`.

## 2. Construction, singleton, and real registrations

`Get` (`0094AC43`) returns the same static object at `00B634D0`, without AddRef.
On the first call (bit 0 of `00B6351C` clear), it sets that bit **before** constructing
the object, constructs in place, and registers exit callback `00A7F63C`. Later calls
skip both actions. There is no lock, wait, or atomic instruction in this path; concurrent
construction is not promised. A recursive call during construction observes the
already-set flag and returns the same storage without waiting for completion.
Exception unwind **does** clear the flag: handler `00A6F214` uses descriptor
`00B03278`, whose state-0 unwind entry at `00B03270` names `00A6F206`; that cleanup
clears bit 0 only. Constructor handler `00A6F1FC` uses descriptor `00B03254`, with
state-0 cleanup `00A6F1F4` invoking base destruction. Neither descriptor has a catch
table. An exception after the base constructor has completed destroys that base,
propagates, and permits a later getter call to try construction again. Exit-handler
registration's own result is ignored. This answers TD3 for the observed handlers.

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
The exact native interface identities are the numeric QI IDs below. No semantic
interface names are established by this evidence; names are not prerequisites for
the slot, ownership or error contracts.
The calculation is unsigned-zero-extension of the old word, multiplication by 2,
then signed integer division by 8 (all inputs here nonnegative), before the new
format is written. This is not a complete decoder specification.

### 5.1 Small service: creation and reference lifetime

Its method at +0C (`0094AB03`) takes `(path,iid,out)` and pops 12 argument bytes.
It copies the supplied path into a temporary string, allocates and constructs a
**fresh** large service, invokes that service's +0C with the copied character path,
then invokes its QueryInterface with `(iid,out)` and returns that QI's AL result.
It neither examines the open result nor Release-cleans the newly created object
when QI fails. The small service's list is not consulted or modified by this method:
there is no cached-instance lookup or retained-child ownership in this path.
Allocation-null is not handled after allocation: the following virtual call
dereferences the null pointer. Successful QI yields the new object's first reference.
An open failure can therefore coexist with a successful interface result.

The small service's +04 is AddRef (`00968B60`, increments +4), +08 is Release
(`00892A51`), and +10 is its deleting destructor. A Release starting at 1 calls
the destructor with flag 1 and returns 0. Every other count is decremented modulo
2^32, including 0 becoming `FFFFFFFF` without destruction. List construction makes
an empty circular sentinel (`004F5A00`); destruction (`005650A0`) frees list nodes
and sentinel, with no virtual Release on node payloads. The shipped creation method
leaves this list empty.

### 5.2 Large service: interface and slot surface

QI (`0096C390`) accepts ID 1 and `3AFD825F`, returns the base pointer and AddRefs;
unsupported IDs return false without touching `out`. AddRef increments +4.
Release (`0094A5C0`) starting at 1 first stores zero, invokes its +54 deleting
destructor with flag 1, and returns 0. Otherwise it decrements modulo 2^32, including
zero underflow. Destruction (`0094AAB8`) installs its own table, calls close below,
destroys the path string, then the embedded file; +54 wrapper `0094AA9C` frees the
object only when flag bit 0 is set and returns the original pointer.

Offsets are within `00AD4ED8`. Stack sizes below exclude `this` and the return address.

| Slot | Native body | Arguments/result and behavior |
|---|---|---|
| +0C | `0094AEAE` | 4 bytes, path. Clear override-valid via +58; copy path to +10D4; call +1C and return its AL. Does not first close an already active stream. |
| +10, +18 | `0093878E` | 4 bytes each; return false AL, no side effects. |
| +14 | `0094A977` | 4 bytes, resource key pointer; resource-to-path chain below; returns AL. |
| +1C | `0094AC88` | No stack arguments; initialize/open current path; AL. |
| +20 | `0094A907` | No stack arguments; close/release resources; always true AL. |
| +24 | `0094A5DB` | No arguments; tail-forward to embedded file +10 (`004770F0`), returning raw byte file+18 in AL; no normalization. |
| +28 | `0094A5E3` | Return raw 32-bit +1100 estimate in EAX. |
| +2C | `0093877E` | 8 bytes; return false AL, no side effects. |
| +30, +34 | `0094A5EA` | No arguments; return false AL, no side effects. |
| +38 | `0094A5ED` | 8 bytes, `(destination,byteRequest)`; PCM delivery below; EAX byte count. |
| +3C | `0094A81A` | No arguments; rewind/skip ID3 below; AL. |
| +40 | `0094A892` | Return borrowed pointer to format at +10EC; no copy or AddRef. |
| +44 | `0094A899` | Return byte +10E8 if +10E9 nonzero, otherwise false AL. |
| +48 | `0094A8AC` | 4 bytes; copy argument's low byte to +10E8, set +10E9=1, return true AL. No normalization of the copied byte. |
| +4C | `0094A8CA` | No arguments; tail-forward to embedded file +18 (`00918345`), returning logical position dword file+48 when file+18 is nonzero, otherwise FFFFFFFF. |
| +50 | `0094A8D2` | 4 bytes, offset; zero calls +3C then returns false regardless; nonzero requires embedded file +10 true, then calls file +30(offset), returning EAX=1 only if the returned value equals offset, otherwise 0. No decoder-buffer reset on the nonzero path. |
| +54 | `0094AA9C` | 4 bytes, deleting-destructor flags. |
| +58 | `0094A8C2` | No arguments; clear +10E9; no defined return value. |

The +14 path constructs its temporary system-service reference (`004496C0`), then
calls that interface's +80 with `(key,&object)`. On success it QIs the object for
`856C8CF7`; on that success it calls the obtained interface's +48 `(key,&string)`.
If this produces a path, it calls this large service's +0C and returns its AL.
Failure at any of these Boolean steps returns false. All obtained nonnull temporary
interfaces are Released and the temporary string/reference destroyed along the
normal return paths, including callbacks that return false with nonnull output.
The service pointer and Boolean-success outputs are dereferenced without independent
null checks. `004496C0` calls `RZGetFramework`, then framework +14 with
`(056B906E,656B8EFC,&service)` if the framework exists (`004496D4–004496E1`). These
are observed request IDs; semantic interface names are not established. No framework/service
leaves a null pointer that this +14 path subsequently dereferences.

The embedded object's constructor call is `0094A4F9 -> 00918B96`; that constructor
installs table `00ABFED0`. Thus the forwards above use cRZFile endpoints, despite
the shared four-byte body `004770F0` having an unrelated exported owner name.
Both thunks add exactly +20 to ECX and jump through the file table, preserving
the caller's stack. They acquire no reference and return no owned object.
The +4C position is a cached logical position, not a direct OS seek/tell request.
Native QI comparisons are at `0094A457–0094A479` (1/FCEFF549) and
`0096C390–0096C3AF` (1/3AFD825F); neither yields a textual interface name.
Other file calls used here are +1C=`009186F5` (length), +30=`00918374`
(absolute seek), +38=`00918382` (exact-count Boolean read). The latter calls
file+3C with a local count and returns true only if that operation succeeds AND
its resulting count equals the request; a short read can have copied bytes and
advanced the file while returning false to this decoder wrapper.
Evidence: native table dwords `00ABFEE0/00ABFEE8/00ABFEEC/00ABFF00/00ABFF08`,
forwarders `0094A5DB–0094A5E2`, `0094A8CA–0094A8D1`, and endpoint instructions
`004770F0–004770F3`, `00918345–00918351`, `00918382–009183AC`.

### 5.3 Open, close, and partial-failure ownership

Open (+1C, `0094AC88`) gets the extension of the current path, lowercases it, and
compares against the static string initialized to `.mp3` by `00A793D6`
(`00A92EA8` literal; static object `00B63524`). A mismatch invokes +48(false),
returns false, and does not free/close any preexisting stream. Comparison includes
the dot. Matching extension with +10D0 already nonzero returns true without
replacing resources or recalculating metadata. Consequently assigning a new path
through +0C while active does not itself switch the open file.

For an inactive matching path, set active +10D0=1 **before allocation**. Destroy
and CRT-free any old decoder at +A0, allocate `4A60` hex bytes by CRT malloc, run
decoder construction `00A667B4` and reset `0094A2AB`, store its pointer, then reset
it again. Reset clears decoder +4A48 and +4A50 and sets byte +4A5C=1; it establishes
no return value. Allocate two independent `2000` hex-byte PCM buffers into +A4/+A8;
clear pending/current frame counts +10BC/+10C0; assign the embedded file's path via
file +70, and call file +0C with arguments `(1,2,1)` in that order. A false AL from
file open returns false **without clearing active or freeing allocations**. A retry
of +1C then returns true from the active branch, without another file-open call.
Decoder allocation-null is unchecked before the second reset, and PCM malloc
results are not checked here. No recoverable allocator-failure policy is promised.

After successful file open, call +3C and ignore its result. If override-valid
+10E9 is zero, call +38 with a 4-byte scratch buffer and request 4 bytes; set the
override through +48 according to whether the read's EAX is nonzero, then call +3C
again. This probe's underlying decode behavior is subject to TD1.

The +1100 estimate is then computed from full file size (embedded file +1C),
parsed bitrate +1C, and format average-byte-rate +10F4. The exact instruction order
at `0094AE1F–0094AE68` is: form the signed-32-bit wrapped value `(bitrate << 10)`;
convert it with x87 integer load and store to binary32; form the unsigned-32-bit
wrapped `(fileSize << 3)` and load to x87, adding exactly 2^32 for a negative signed
interpretation; divide by the stored binary32 denominator; load unsigned +10F4 by
the same signed-load/2^32 correction; multiply in x87; convert with `_ftol2` and
store low EAX. It is not simply fileSize times a rational number in double precision.
The conversion behavior is specified immediately below. The full file size includes any ID3 prefix.
After this sequence open returns true. There is no check for zero bitrate, failed
probe/rewind, or a nonsensical estimate.

`_ftol2` here is native body `009EEF04–009EEF78`, not a signed-32-bit conversion
or saturating cast. It duplicates ST0, stores one binary32 sign copy and converts
to a **signed 64-bit** stack temporary using x87 FISTP. It does not change the x87
control word. For finite results whose conversion/correction is representable,
it corrects the rounded integer toward zero and returns EDX:EAX. The caller stores
only EAX at +1100. Consequently values outside signed/unsigned 32-bit range but
within the signed-64-bit conversion range wrap to their low 32 bits; e.g.
4294967297.75 stores 1 and -1.75 stores FFFFFFFF. No range/error check follows.

With invalid-operation masked, NaN, either infinity, and finite values whose
FISTP conversion is outside signed-64-bit range write integer-indefinite
`80000000:00000000`. The low-word-zero branch tests `high&7FFFFFFF`, sees zero,
pops the two remaining x87 values and returns that pair unchanged; the service
stores **00000000**, not 80000000, at +1100. The same branch also handles integer
zero and exactly -2^63. Sticky exception flags are not cleared. With relevant x87
exceptions unmasked there is no guaranteed normal return/store: the helper and
preceding divide may raise the corresponding floating exception. This is a
control-word-dependent native contract, not a claim about a runtime default.
Evidence: FISTP qword `009EEF13`, correction paths `009EEF27–009EEF61`,
zero/indefinite branch `009EEF63–009EEF78`, low-dword store `0094AE68`.

Close (+20, `0094A907`) does nothing if +10D0 is zero and still returns true.
Otherwise it calls embedded file +14, ignores the result, CRT-frees and nulls each
PCM buffer +A4/+A8, destroys (`0094A972` -> `00A66BBE`) and CRT-frees/nulls +A0,
then clears active +10D0. It does not reset the estimate, format, override, pending
counts or path. Decoder destruction reinstalls its table and, only if decoder byte
+4A38 was nonzero, delegates cleanup to `00A66BA7`. That helper clears +4A3C and
+4A38 only if +4A38 was nonzero and always returns EAX=0; it does not free memory.
CRT allocations are paired with CRT free, whereas the outer object
is paired with `GlobalHeapFree`.

### 5.4 Rewind and ID3 prefix handling

Rewind (+3C, `0094A81A`) clears current/pending frame counts, calls embedded file
+24 (reset), then file +38 with `(scratch,10)`. False AL returns false directly.
On success `0094A3F8` examines the first three bytes only. Unless they are ASCII
`ID3`, it returns 0; otherwise it returns
`10 + (((b6 * 128 + b7) * 128 + b8) * 128 + b9)`, using zero-extended bytes and
32-bit arithmetic. It does not mask the high bits of these size bytes or validate
version/flags. Thus `ID3` with size bytes 0,0,0,128 gives length 138, rather than 10.

Store this length at +10B8. Length zero calls file +24 again, putting non-ID3 data
back at the beginning. A nonzero length consumes `length-10` one-byte file reads
(signed-positive loop bound); individual Boolean results are ignored. The method
then returns true, even if these skip reads failed. An ID3 prefix of size zero
leaves the cursor at 10. Rewind does not reset the decoder object or override.

### 5.5 PCM delivery boundary

Read (+38, `0094A5ED`) computes its requested frame count by signed integer division:
first zero-extend the stored 16-bit channels and bits/sample, multiply in 32 bits,
divide by 8; then divide signed `byteRequest` by that result. Zero divisor faults
are unchecked. There is no active/file-open/destination-null precondition check.
It clears this call's frame counter +10C0 and first copies pending 16-bit samples
from +10C8 (left) and, only for stored channels=2, +10CC (right), advancing each
used pointer by 2 per delivered frame. Output is interleaved left/right.

While more frames are requested, it reads a 4-byte compressed header through file
+38, parses it with `0094A2C1` into six words at +8..+1C, and stops on a false file
read or zero parsed layer. It then reads `parsedFrameBytes-4` more bytes into its
inline compressed buffer, stopping on false AL. It submits the complete frame to
`0094A079` with limit -1 (the helper return is ignored), assigns the two PCM buffers
as output planes, and calls `0094A0E4` with parsed samples/frame as the limit.
That helper's EAX becomes the new decoded-frame count and contributes to total
available frames; decoding/error/concealment details remain TD1. It resets the
plane cursors to buffer starts, aliases right to left for mono, then copies 16-bit
samples until this decoded block or the request is exhausted. The new-block copy
tests parsed channels at +14, whereas pending-block copy tests stored format channels.

At exit, pending +10BC is total available frames minus delivered frames. The format
is rewritten from the latest parsed fields: channels=low16(+14), sample rate=+18,
PCM tag=1, bits/sample=16, block alignment=channels*2, average byte rate=alignment
times sample rate; 32-bit products wrap and stored word fields truncate. Clear +10FC.
Return EAX=delivered frames times this **new** alignment, not the original request
size and not a Boolean. Unaligned byte requests truncate to whole requested frames;
an error after some frames returns a short byte count. This body has no frame-size
bound check before reading into its inline buffer. Behavior before any valid
header has initialized the parsed words is not a defined successful empty stream;
those words are not initialized by `0094A4D8`.

### 5.6 Compressed-header parser

`0094A2C1` is caller-cleaned, taking `(fourReadableBytes,sixWordOutput)`. It first
clears all 24 output bytes. It returns EAX=-1 if byte 0 is not FF, top three bits
of byte 1 are not all set, version `(b1 >> 3) & 3` equals 1, layer bits
`(b1 >> 1) & 3` equal 0, bitrate index `b2 >> 4` equals 15, or sample-rate index
`(b2 >> 2) & 3` equals 3. It does not reject bitrate index 0 or check CRC/payload,
version flags beyond those tests, or emphasis. Failure leaves six zero words.

Layer number is `4-layerBits`. Version group is 0/1/2 for version bits 0/2/3;
lower-rate flag is 1 for groups 0/1, otherwise 0. Native sample-rate table
`00AD4D30` has these values indexed by group and sample-rate index:

| Group | Index 0 | Index 1 | Index 2 |
|---|---:|---:|---:|
| 0 | 11025 | 12000 | 8000 |
| 1 | 22050 | 24000 | 16000 |
| 2 | 44100 | 48000 | 32000 |

Bitrates below are indexed 0..14 by `b2 >> 4`. Native row start is
`00AD4D1C + 4*15*row`; index arithmetic is verified at `0094A35D–0094A3D1`.

| Row / applicability | Values for indices 0..14 (kb/s) |
|---|---|
| 1: group 2, layer 1 | 0,32,64,96,128,160,192,224,256,288,320,352,384,416,448 |
| 2: group 2, layer 2 | 0,32,48,56,64,80,96,112,128,160,192,224,256,320,384 |
| 3: group 2, layer 3 | 0,32,40,48,56,64,80,96,112,128,160,192,224,256,320 |
| 4: groups 0/1, layer 1 | 0,32,48,56,64,80,96,112,128,144,160,176,192,224,256 |
| 5/6: groups 0/1, layers 2/3 | 0,8,16,24,32,40,48,56,64,80,96,112,128,144,160 |

Padding is `(b2 >> 1) & 1`. For layer 1, frame bytes are
`(truncate(12000*bitrate/sampleRate)+padding)*4`, samples/frame=384. Otherwise first
truncate `144000*bitrate/sampleRate`; only for layer 3 with lower-rate flag set,
arithmetic-shift the quotient right by one; then add padding. Samples/frame=576
in that reduced layer-3 case, otherwise 1152. Division is signed 32-bit integer
division on nonnegative table inputs; padding is added after division/halving.
Channels=1 only when top two bits of byte 3 equal 3, otherwise 2. Output words are
layer, frame bytes, samples/frame, channels, sample rate, bitrate; return EAX=0.
Free-bitrate index 0 can produce zero frame bytes with a nonzero parsed layer.

This parser differs from internal decoder parser `00A667FE` (§5.9), including
acceptance and mutation on failure. They must not share one assumed validation rule.

### 5.7 Compressed-input submission boundary

`0094A079` takes decoder `this` in ECX, then `(buffer,byteCount,sampleLimit)` as
three 32-bit stack arguments (callee pops 12). A signed-positive pending-byte count
at +4A48 returns -1 without writes. Otherwise signed byteCount<4 returns -2 without
reading or writing the buffer/submission fields. For byteCount>=4 it stores the
borrowed buffer pointer at +4A44, assembles its first four bytes in big-endian
order, and calls `00A667FE`. A negative parser result becomes -2; pointer/parser
side effects are not rolled back. Otherwise store sampleLimit at +4A4C, byteCount
at +4A48 and return 0. It does not copy/own the buffer or validate compressed-frame
length. Parser acceptance and writes are specified in §5.9.

### 5.8 Layer-3 side-information parser

`00A675E0` takes the decoder in ECX, no stack arguments; AL=1 means success and
AL=0 means the explicit invalid-block-type failure. Upper EAX is not a Boolean
contract. Its only direct caller is `00A69EC2` inside frame driver `00A69E86`;
this is reachable decoder code, retained in the attributed scope.

Let B be the borrowed byte pointer at +24A4 and t the signed 32-bit bit cursor at
+24A8. Each n-bit extraction loads **four** bytes at `B+arithmeticShift(t,3)`,
interprets them big-endian, logically shifts right by `32-(t&7)-n` (native shift
count masked to five bits), masks to the low n bits and advances t by n modulo
2^32. Bit order is most significant first. Native mask table `00AF3560` contains
`2^n-1` for these widths. Neither a byte count nor end pointer is consulted.
Readable padding is required through the last four-byte load even when the
logical fields fit in fewer bytes; short input does not produce a clean partial
read result. This body neither allocates nor frees any memory and retains no new
pointer. Fields omitted by the selected branch retain their previous contents.
Evidence: first load/extraction `00A675E8–00A67637`, masks and cursor stores
throughout `00A675E0–00A6832C`.

Channels C is the signed dword at +4A40. Full rate means byte +4A0E=0; any other
byte selects reduced rate. Read the common prefix in the following order:

| Field | Full-rate width | Reduced-rate width | Stored at |
|---|---:|---:|---|
| main-data backward offset | 9 | 8 | +34CC dword |
| private bits | 5 if C=1, otherwise 3 | 1 if C=1, otherwise 2 | +34D0 dword |
| four scale-factor-selection flags, channel order | four separate 1-bit reads per channel | absent | +34D4+channel*A0h, then +4,+8,+C |

After that prefix, full rate iterates granule g=0,1 outside channel c=0..C-1;
reduced rate iterates only g=0. Each record starts at
`R=this+34E4h+c*A0h+g*48h`; all listed fields are zero-extended dword stores.
For C<=0 the channel loops are skipped after reading the two common fields;
full rate still visits both empty granule iterations. C>2 is unchecked and can
write beyond the intended two-channel layout.

| Read order / condition | Width | Record offset and descriptive field |
|---|---:|---|
| always | 12 | +00 part2/3 bit length |
| always | 9 | +04 big-value pair count; values 289..511 are accepted here |
| always | 8 | +08 global gain |
| always | 4 full rate, 9 reduced rate | +0C scale-factor compression |
| always | 1 | +10 window-switch flag |
| switched | 2, then 1 | +14 block type, +18 mixed-block flag |
| switched | 5,5 | +1C,+20 Huffman table selectors 0,1; selector 2 at +24 retained |
| switched | 3,3,3 | +28,+2C,+30 subblock gains |
| unswitched | 5,5,5 | +1C,+20,+24 Huffman table selectors |
| unswitched | 4,3 | +34,+38 region counts 0,1; set +14 block type to zero |
| full rate, after either branch | 1 | +3C preflag |
| both rates, after either branch | 1,1 | +40 scale-factor scale, +44 count1-table flag |

In a switched record, only **after** reading all three subblock gains, reject
block type zero with AL=0. No cursor or prior store is undone; suffix flags,
derived region counts, subsequent channels and granules are not processed.
Otherwise full rate sets region0=8 for block type 2 with mixed=0, and 7 for all
other switched records, then sets region1=20-region0. Reduced rate sets region0=8
for type 2/mixed=0 and **leaves region1 unchanged**; all other switched records
set region0=7, region1=13. Reduced rate never writes preflag. Unswitched records
do not clear the retained mixed-block flag or subblock gains. This parser makes
no other semantic validation, including no bounds check on big-value count,
selectors or region totals. Successful completion returns AL=1.
Evidence: full-rate branch/check/derived counts `00A67940–00A67C66`, reduced-rate
counterpart `00A67F5E–00A6828B`, suffixes/returns `00A6828B–00A68328`.

For normal C=1/2 and initial cursor t, success consumes respectively 136/256 bits
at full rate and 72/136 bits at reduced rate, independent of switch flags.
These are 17/32 and 9/17 logical bytes for byte-aligned input, plus the readable
padding required by the extraction rule. No sample output is produced here.

### 5.9 Internal header and reset contracts

`00A667FE` receives decoder ECX and one big-endian assembled header dword H;
it pops four bytes and returns full EAX=0 or -1. It owns no buffer and performs
no payload read. First require `(H & FFF00000h)=FFF00000h`; failure returns -1
without decoder writes. This requires bit20=1, so the later MPEG-2.5 branch is
unreachable through this entry. MPEG-2.5 accepted by §5.6 is rejected here.
No separate rejection exists for layer bits zero, sample-rate index3, or bitrate
index15. On a passing prefix it makes these stores before subsequent failure tests:

| Decoder field | Value |
|---|---|
| byte +2490 | L=4-((H>>17)&3), including L=4 |
| bytes +4A11,+4A12,+4A13 | (H>>19)&1, (H>>16)&1, bitrate index K=(H>>12)&15 |
| bytes +4A15,+4A16,+4A17 | padding (H>>9)&1, channel mode (H>>6)&3, mode extension (H>>4)&3 |
| bytes +4A18,+4A19 | (H>>3)&1, (H>>2)&1 |
| bytes +4A0D,+4A0E | 0, then reduced flag F=1-((H>>19)&1) |
| byte +4A14 | rate index I=((H>>10)&3)+3*F |
| byte +4A0F | ((H>>10)&3)+3*(1-F) |
| dwords +2484,+2488 | sample-rate table[I] |
| dwords +4A40,+4A1C | C=(mode==3 ? 1 : 2), then 32*C |

Native rate dwords at `00AF3538`: 44100,48000,32000,22050,24000,16000,
11025,12000,8000. Thus index3 spills into the next group: F=0 selects22050;
F=1 selects11025. No division-by-zero results from these reachable indices.
K=0 now emits the native diagnostic at `00AF35E4` and returns -1, retaining all
above writes and the old +2494/+2498. Otherwise load a **signed 16-bit** bitrate
from `00AF3458+2*(16*(3*F+L)+K)` and store sign-extended at +2494. Rows1..6 are
the corresponding bitrate rows in §5.6, with a final zero at index15. The extra
reachable row7 (F=1,L=4) is the adjacent rate-table words interpreted as signed
halves: -21436,0,-17536,0,32000,0,22050,0,24000,0,16000,0,11025,0,12000,0.
No invented valid MPEG layer is assigned to that spill.

L=1 returns -1 **after** writing bitrate, leaving old +2498. For every other L,
including4, multiply bitrate by144000 as a wrapping signed dword, signed-divide
by +2484 (truncate toward zero), and store at +2498. For L=3 and F!=0, arithmetic
shift that quotient right1 and store again. Then add padding and subtract4,
with dword wrap, and return0. +2498 counts payload bytes excluding the four-byte
header; zero bitrate from K=15 is accepted and yields -4 or -3. No failed parse
clears the decoder state. Evidence: acceptance `00A66806–00A66816`, stores/tables
`00A6681B–00A66985`, division/order/returns `00A6698B–00A669CA`.

The neighbor-attributed `00A66A31` remains in scope. It reads four bytes at
borrowed +24A4 when nonnull, otherwise the raw dword at +4A3C; byte-swaps the
dword and calls `00A667FE`. It returns EAX=1 for parser result -1, otherwise0
(an inverted failure predicate), retaining parser side effects. No byte-count
or partial-input check. Evidence: `00A66A31–00A66A5E`; there are no direct callers
in the exported caller query, which does not establish that indirect use is absent.

`00A667DA` zeroes exactly 900h bytes starting at decoder+4 if global dword
`00BB46C4` is nonzero, otherwise1200h bytes. It owns/frees nothing, performs no
size check, and does not clear the following byte. EAX is memset's destination
result, not a decode-success code (`00A667DA–00A667FD`). `00A66B80` copies +249C
to +4A34 and clears four dwords +24AC,+24A8,+4A30,+24B4, returns EAX=0 and leaves
all other state intact (`00A66B80–00A66BA6`). Neither reset means complete object
initialization or ownership transfer.

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
| TD-V13 | Small-service +0C: force open failure, query supported/unsupported IID in separate cases | Supported QI can still return true; unsupported QI returns false without destroying the stream |
| TD-V14 | Directly Release a fresh large service at count0 | Stored count and EAX become FFFFFFFF, no destruction |
| TD-V15 | Path x.MP3: force file-open false, then call +1C again | First false retains active/allocations; second true without another file open |
| TD-V16 | Assign a different matching-extension path via +0C while active | String changes and override clears; old file/resources remain active |
| TD-V17 | Open a nonmatching path while active | False; explicit override=false; existing resources retained until close |
| TD-V18 | Rewind: successful ID3 ten-byte read, size bytes 0,0,0,128, subsequent reads fail | Length138, 128 skip attempts, final true |
| TD-V19 | Seek +50 to zero with successful rewind | Reset occurs; method returns false |
| TD-V20 | Set override byte7; query; clear via +58; query | Returns byte7, then false; stored value need not be erased |
| TD-V21 | Parse FF FB 90 00, then FF F3 80 C0 | layer3/417bytes/1152frames/stereo/44100Hz/128kbps; then layer3/208bytes/576frames/mono/22050Hz/64kbps |
| TD-V22 | Parse FF FB 00 00 and FF FB F0 00 | Index0 succeeds with layer3 and frameBytes0; index15 fails, clears all 24 bytes |
| TD-V23 | Read request5bytes, stereo/16-bit, at least one pending frame | One whole frame (4bytes); fifth destination byte untouched |
| TD-V24 | Submit with positive pending bytes; repeat with count3 and no pending bytes | Returns -1 then -2; no input dereference/submission-field writes |
| TD-V25 | Throw during first getter construction after base completes, catch outside, call getter again | Base cleanup, flag clear, then construction retry |
| TD-V26 | Side info: full rate, C=1, cursor0, zero bytes with readable padding; record storage prefilled nonzero | AL=1, cursor136; two unswitched records; block types/region counts/flags zero; mixed flags and subblock gains retained |
| TD-V27 | Side info: reduced rate, C=2, cursor0, zero bytes with readable padding; preflags prefilled 7 | AL=1, cursor136; one record/channel, preflags remain7; no scale-factor-selection flag writes |
| TD-V28 | Full-rate mono prefix18 zero bits, then record fields 12/9/8/4 bits zero, switch1, type0, mixed0, selectors0/0, gains0/0/0 | AL=0 at cursor74; prefix and record through gains retained; derived counts and three suffix flags untouched; second granule untouched |
| TD-V29 | Reduced-rate mono: prefix9 zero bits; 12/9/8/9 zero bits, switch1, type2, mixed0, selectors0/0, gains0/0/0, suffix0/0; prefill region1=99 and preflag7 | AL=1, cursor72; region0=8, region1 remains99, preflag remains7, selector2 retained |
| TD-V30 | Reduced-rate mono, valid unswitched record with big-value count511 and region counts15/7 | AL=1, cursor72; stores511,15,7 without rejection |
| TD-V31 | Synthesis 00A6A25B: phase0, channels0, both stack pointers null | Phase becomes15; no transform/output call or pointer read; no Boolean return contract |
| TD-V32 | Synthesis: one channel and readable packed input/history, PCM destination null | Phase advances, transform/history writes still occur, PCM window/output stores skipped |
| TD-V33 | Internal header FFFB9000; repeat FFFB9C00 | EAX0; +2494=128; sample rate44100/22050; payload413/831; both stereo |
| TD-V34 | Internal header FFFBF000 with old payload99; repeat FFFB0000 with old payload99 | First EAX0, bitrate0, payload-4; second EAX-1, common header fields updated but old bitrate/payload retained |
| TD-V35 | Internal header FFE380C0 (MPEG-2.5) with sentinel-filled decoder | EAX-1, decoder untouched; §5.6 accepts this version |
| TD-V36 | Direct _ftol2 inputs NaN,+infinity,-infinity,2^63 with invalid masked; examine caller's store | Each produces EDX:EAX=80000000:00000000, and +1100=0; invalid flag is retained |
| TD-V37 | Direct conversion4294967297.75, then -1.75 under round-to-nearest with exceptions masked | Caller stores1, then FFFFFFFF; neither is saturated to32-bit range |
| TD-V38 | File-open byte7 and logical position123, then open byte0 | Large +24 returns AL7 then0; +4C returns123 then FFFFFFFF; no AddRef or OS tell |
| TD-V39 | History-clear helper with nonzero global feature flag and storage filled AA; repeat flag0 | Clears [this+4,this+904h) then [this+4,this+1204h); first following byte remains AA |

## 7. Open contracts and coverage boundary

Vector TD-V23 assumes parsed channel/rate fields matching the stored stereo format;
it is a pending-data case after a valid header. The 39 vectors are independent
review fixtures, not reports of execution against an implementation or the game.

Checklist reconciliation: D=6 bodies/294 B; S=35 bodies/7015 B; U=28 bodies/23963 B;
A=1 artifact/21 B. D/S are documented boundaries, not proof that nested decoder
arithmetic is specified. TD1 and TD4 remain open; TD2 is answered in §5.2/§5.3,
with numeric interface identities retained; TD3 is answered in section 2
and archived in answered/tools-com-director.md. Whole-owner promotion is withheld.

See [tools-com-director questions](questions/tools-com-director.md). The full attributed
checklist is retained so that documenting the four director-specific bodies cannot
accidentally award all 31,293 bytes. Startup/lifecycle hooks are confirmed for the
Windows tables; no other-platform `OnStart`/`PostAppInit` behavior is imported.
