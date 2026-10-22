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
| `00420036` | 21 | U; distant reached helper |
| `0094A079` | 107 | U; decoder input helper |
| `0094A0E4` | 455 | U |
| `0094A2AB` | 22 | S; decoder reset boundary (§5.3) |
| `0094A2C1` | 311 | U |
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
No stable external class/interface names are asserted for these services here.
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
| +24 | `0094A5DB` | Tail-forward to embedded file +10, with adjusted `this` (+20). |
| +28 | `0094A5E3` | Return raw 32-bit +1100 estimate in EAX. |
| +2C | `0093877E` | 8 bytes; return false AL, no side effects. |
| +30, +34 | `0094A5EA` | No arguments; return false AL, no side effects. |
| +38 | `0094A5ED` | 8 bytes, `(destination,byteRequest)`; PCM delivery below; EAX byte count. |
| +3C | `0094A81A` | No arguments; rewind/skip ID3 below; AL. |
| +40 | `0094A892` | Return borrowed pointer to format at +10EC; no copy or AddRef. |
| +44 | `0094A899` | Return byte +10E8 if +10E9 nonzero, otherwise false AL. |
| +48 | `0094A8AC` | 4 bytes; copy argument's low byte to +10E8, set +10E9=1, return true AL. No normalization of the copied byte. |
| +4C | `0094A8CA` | Tail-forward to embedded file +18 with adjusted `this`. |
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
null checks. Semantic names and the dependency service ID are left in TD2.

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
The positive finite in-range conversion truncates toward zero; invalid/overflow
conversion behavior is part of TD2. The full file size includes any ID3 prefix.
After this sequence open returns true. There is no check for zero bitrate, failed
probe/rewind, or a nonsensical estimate.

Close (+20, `0094A907`) does nothing if +10D0 is zero and still returns true.
Otherwise it calls embedded file +14, ignores the result, CRT-frees and nulls each
PCM buffer +A4/+A8, destroys (`0094A972` -> `00A66BBE`) and CRT-frees/nulls +A0,
then clears active +10D0. It does not reset the estimate, format, override, pending
counts or path. Decoder destruction reinstalls its table and, only if decoder byte
+4A38 was nonzero, delegates cleanup to `00A66BA7`; the latter's subobject contract
remains TD1. CRT allocations are paired with CRT free, whereas the outer object
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
