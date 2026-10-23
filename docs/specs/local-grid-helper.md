# Local grid helper (`cSC4LocalGridHelper`)

Status: **bounded native behavior specification**, 2026-10-23,
`re-local-grid-helper4`. The helper's dispatch, argument construction and normal
lifecycle are specified here. Foreign resource creation, rendering, effect
internals and an ambient floating-point environment are separate contracts.
No native execution or portable implementation output was used.

## 1. Scope and primary evidence

This is the helper embedded by city placement tools, rather than the tools'
choice of when to show or move it. Existing consumer facts are inputs from
[city-tools-input.md](city-tools-input.md) and [zone-tool.md](zone-tool.md).
[effects.md §4](effects.md#4-creating-an-effect) supplies names for foreign
effect operations; this spec does not reimplement that manager.

Evidence follows the canonical export/instruction workflow in
`docs/re-workspace.md`. Addresses are Windows image-base `0x00400000` addresses
in the supplied SimCity 4 Deluxe 1.1.641 working PE. SHA-256:

| Read-only evidence | SHA-256 |
|---|---|
| `bin/SimCity4.exe`, also recorded by the export | `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b` |
| `ghidra/export/windows/export-summary.txt` | `8f75d6ec79b89d8d583204605e1abfb9d2daa2acb87ccfed64438190271fd514` |
| `instructions.asm` | `31b87fdd94db47d22d1b5af1863dae33706b93d0867b766e5a82117e14659416` |
| `functions.tsv` | `ffada4c2a05f6e99dc446d9d830da084eb94c4562fd7f10cebe9f7eb27400c6d` |
| `function-details.tsv` | `62764f191a75eaeaadf665c66aa3d238bf6931b82d2d900f6c5f13b1f78d558f` |
| `calls.tsv` | `7f927b2dc8d698eae663d5ddd1afde8e2b5ff688ae6937ac141a63b1e7a32208` |
| `symbols.tsv` | `c2442038e1a95ccb74b23e8f4cbe6a6510fa576efbd81f6fcd3962cfb5c767cd` |

`re_query.py func/asm --bytes/callers/search` was queried first. Numeric rules
below come from instructions and PE constants, not pseudocode. Local decompilation
was navigation only: notably, it omits the transform construction and loses the
double stores in this body. No live REA query was necessary.

### 1.1 Entry and provenance ledger

**Supported** means proved helper-owned operations in the valid normal domain.
**Partial** marks a bounded signature/provenance claim. **Foreign** marks an
independent primitive, never a whole-helper oracle. **Unknown** marks a fact
that this publication does not establish.

| Entry and inclusive primary range | Body qualification | Machine signature and return qualification |
|---|---|---|
| Constructor `0x005FB290–0x005FB298` (9 bytes) | Supported: zero the first pointer word. Partial: ownership of this tiny generic body, not exclusivity. | `ECX` is storage; no stack arguments; `EAX` returns that storage address. No vptr write. |
| Destructor-labeled entry `0x00456A20–0x00456A2B` (12 bytes) | Supported: null-test and release-only dispatch. Partial: generic auto-matched cleanup, not proof of a unique class destructor implementation. | `ECX` is storage; no stack arguments; no defined result. Tail-dispatches Release when nonnull. |
| `Shutdown`, `0x005FB2A0–0x005FB2CE` (47 bytes) | Supported: active query, optional Stop(0), clear-before-release. | `ECX` is helper; no stack arguments; no defined result. |
| `ShowEffect`, `0x005FB2D0–0x005FB31A` (75 bytes) | Supported: active-state-controlled Start/Stop. | `ECX` is helper; one four-byte stack slot, only its low byte tested; `RET 4` or equivalent foreign tail return. No defined result. |
| `Init`, `0x005FB340–0x005FB357` (24 bytes) | Supported: conditional resource request with direct field output. | `ECX` is helper; no incoming stack arguments; no defined result. Factory status is not tested. |
| `SetPosition`, `0x005FB360–0x005FB4D2` (371 bytes) | Supported: guard, five scalar requests, two floor conversions, transform construction and dispatch. Partial: arbitrary-environment exact FP results. | `ECX` is helper; one four-byte stack pointer to three binary32 components, consumed with `RET 4`; no defined result or output buffer argument. |

These are six non-thunk entries in the export, totaling the provisional 538
bytes. Disjoint ranges and direct bodies establish operations at these addresses;
names, size and `thunk=false` alone do not prove unique ownership. Constructor,
Init and SetPosition each have four direct callers named PlaceLot, Zone,
NetworkIntxTool and NetworkTool. Destructor references include Zone and
NetworkTool destruction, unnamed cleanup and unwind entries. This corroborates
embedded-helper use without settling identical-code folding or C++ declarations.
There is no region-named consumer in this bounded direct-call evidence. There
is no claim of global absence of indirect consumers. No additional owner,
default, byte, native-coverage or runtime-coverage credit follows from this table.

## 2. State, channels and supported domain

The only persistent helper field touched by these bodies is a 32-bit effect
interface pointer at helper offset `+0`. Call it **H**. Constructor sets H to
null. There is no helper vtable initialization, stored position, stored spacing,
orientation, dimensions, initialized flag, visible flag or cached cell in these
bodies. This is a description of observed accesses, not proof that every C++
allocation has exactly four bytes.

| Channel | Qualification and helper-visible contract |
|---|---|
| Global pointer at `0x00B43D1C`, virtual byte offset `+0x1C` | Foreign named-effect request. Arguments are the NUL-terminated bytes `local_grid` at `0x00AA66D0` and a writable pointer to H. Creation, reference acquisition and any factory-side activation remain foreign. |
| Global pointer at `0x00B43CF4`, virtual byte offset `+0x18` | Foreign no-stack-argument scalar accessor, result in x87 ST(0). Denote successive replies `s1…s5`. Used as spacing, offset and scale factors; this lane does not infer its C++ return declaration or a guaranteed constant reply. |
| H virtual `+0x14` | Foreign no-argument active-state query. Only `AL` is tested; nonzero means active for helper branching. |
| H virtual `+0x0C`, `+0x10` | Foreign Start and Stop respectively, each passed a four-byte zero argument. Their result registers are not helper status channels. |
| H virtual `+0x20` | Foreign SetTransform: one pointer to the temporary descriptor in §3.3. Receiving/copying/rendering it is foreign. |
| H virtual `+0x08` | Foreign no-argument Release. No direct AddRef is present in any of the six bodies. |
| `0x009EFF60` | Foreign scalar floor dispatcher: one binary64 stack input, result in ST(0). Preserve its environment-dependent dispatch rather than substituting a host whole-geometry routine (§3.2). |

The normal domain is constructed, live helper storage; valid interfaces and
tables for calls actually reached; a readable, stable three-component position
when H is nonnull; synchronous completed primitives that preserve the helper
and its inputs except for the explicit Init output; finite scalar and position
inputs with positive nonzero divisor replies and finite representable
intermediates. The trace environment is further constrained in §5.

Null H is a positively guarded state, not an error result. Null factory/service
globals, uninitialized storage, invalid interfaces or tables, short/invalid
position buffers and foreign exceptions have no helper recovery contract. In
particular, the SetPosition guard precedes reading the position and service,
but it does not validate those inputs on the nonnull path. Callback mutation,
reentrancy and allocator failure are outside the proved normal domain.

## 3. Owned SetPosition algorithm

### 3.1 Selection, reads and update order

At `0x005FB36C–0x005FB374`, return without foreign calls when H is null.
Otherwise capture the service pointer from `0x00B43CF4` once. All five scalar
calls use that captured interface, even though its vtable is loaded again at
each call. There is no active-state query, Start, Stop, terrain-height/normal
query, orientation selection or bounds clamp.

The argument is one vector **(x, y, z)** of three consecutive binary32 words.
The helper snaps x and z separately and copies y's bits unchanged. It accepts
no width, depth, radius or orientation argument. Unequal rectangle dimensions
are therefore not a native input case for this entry.

The operation sequence is: read x; request s1 and compute the x floor-product;
request s2, then read y and z while completing the x offset/add/store;
request s3 and compute the z floor-product;
request s4 and finish z; request s5 and compute scale/flags; reload H; send one
transform. Primary ranges: `0x005FB3AF–0x005FB425`,
`0x005FB428–0x005FB463`, `0x005FB466–0x005FB4C7`.

There is no equal-input or equal-cell early exit. Every nonnull-H call repeats
all five requests and sends a descriptor, including when the effect is hidden
or the descriptor's semantic values equal those sent previously. SetPosition
does not remember or restore a visibility state and does not write H.

### 3.2 Numeric contract and store boundaries

Constants read directly from this PE are binary64 **1/16** at `0x00AA66E0`,
binary32 **16** at `0x00A8D45C`, and binary32 **1** at `0x00A81228`.
The offset is one sixteenth of a reply, not one half.

Let `R64` and `R32` mean an actual x87 store to binary64/binary32 under the
applicable rounding mode; each arithmetic operation before a store also obeys
the current x87 precision-control/rounding-control setting. This helper neither
sets nor saves that control word. For the finite floor contract, `F(d)` denotes
the scalar dispatcher's ST(0) reply for the binary64 argument d.

| Order | Owned operation and materialization | Instruction evidence |
|---|---|---|
| 1 | Store s1 to binary64 D1. Load x as binary32; divide by D1; store quotient to binary64 Qx for the floor call. | `0x005FB3D9–0x005FB3EE` |
| 2 | Multiply F(Qx) by the **same stored D1**; store product to binary64 Px. | `0x005FB3F3–0x005FB3FE` |
| 3 | Obtain s2; multiply by binary64 1/16; add stored Px; store to binary32 Tx. | `0x005FB402–0x005FB421` |
| 4 | Store s3 to binary64 D3. Load z as binary32; divide by D3; store quotient to binary64 Qz for floor. | `0x005FB425–0x005FB43A` |
| 5 | Multiply F(Qz) by stored D3; store product to binary64 Pz. | `0x005FB43F–0x005FB44A` |
| 6 | Obtain s4; multiply by binary64 1/16; add stored Pz; store to binary32 Tz. | `0x005FB44E–0x005FB45F` |
| 7 | Obtain s5; multiply by binary32 16; store scale to binary32 **without popping** the x87 result. Compare the still-held result with exact 1 to choose flags. | `0x005FB463–0x005FB489` |

The multiply and add in steps 3 and 6 are separate x87 operations; no fused
operation or intermediate binary32 store is present. Steps 1/4 store the
division result to binary64 before floor, so floor of an unrounded quotient
is not equivalent at all boundaries. Likewise the x/z products have explicit
binary64 stores. Steps 3/6 use their own fresh scalar replies, not D1/D3.

When arithmetic is exact, all five replies are the same positive w, and F is
ordinary floor, this reduces to:

- `Tx = floor(x / w) × w + w / 16`;
- `Ty = y`, copied as one 32-bit word;
- `Tz = floor(z / w) × w + w / 16`;
- `scale = w × 16`.

These equations do not remove the operation/store boundaries above. No integer
cell conversion, truncation-toward-zero, nearest-vertex rule or fixed radius is
present. Negative coordinates follow floor in the stated finite floor domain.

The floor dispatcher at `0x009EFF60–0x009F0080` selects an SSE2 path only when
its dispatch word at `0x00BB9D38` is nonzero, `MXCSR & 0x1F80 = 0x1F80`, and
`x87 CW & 0x007F = 0x007F` (`0x009EFF60–0x009EFF98`). Its finite SSE2 path removes fractional bits
and adjusts negative nonintegers downward. Otherwise it delegates to
`lib::__floor_default` at `0x009FB0FA`. The helper does not choose this branch.
The primitive's exceptional-input/error machinery and ambient startup control
state are not specified here; no universal arbitrary-bit result is claimed.

### 3.3 Transform descriptor and flags

One temporary 56-byte descriptor is built at `0x005FB37A–0x005FB3C9` and
`0x005FB46C–0x005FB4C7`, then passed to H's `+0x20` operation. Offsets below
are relative to that descriptor, not the helper.

| Offset | Width | Value delivered |
|---|---|---|
| `+0` | byte | 2 |
| `+1` | byte | 1 when the held x87 scale result compares equal to 1; 3 when unequal or unordered |
| `+2…+3` | two bytes | Uninitialized padding; no deterministic bytes promised |
| `+4…+39` | nine binary32 words | Identity 3×3: `(1,0,0; 0,1,0; 0,0,1)` |
| `+40`, `+44`, `+48` | three binary32 words | Tx, the unchanged y word, Tz |
| `+52` | binary32 | Stored scale from step 7 |

The two flag bytes start at zero. The first is incremented twice; bit 0 of the
second is set unconditionally. Bit 1 of the second is selected by the x87
comparison (`0x005FB470–0x005FB489`), **not** by reloading the stored binary32
scale. A rounded stored scale of 1 alone does not prove flags=1. No renderer
visibility or pixel footprint is established by these bytes. The pointer is
temporary; this publication requires only the synchronous foreign call and
does not promise the descriptor survives the helper's return.

## 4. Lifecycle and resource ledger

### 4.1 Construction and Init

Construction writes null to H and returns the helper address in EAX
(`0x005FB290–0x005FB298`). It does not initialize the effect service, acquire a
reference, request a resource or activate an effect. Reconstructing over a live
retained pointer has no cleanup guarantee.

Init tests H, and when it is nonnull returns immediately. It does not check a
separate initialization flag or refresh/replace an existing pointer. When H is
null, it loads the factory global, passes `local_grid` and **the address of H
itself** as the two arguments, and returns after that one foreign call
(`0x005FB340–0x005FB357`). There is no temporary output, success test, separate
assignment, direct AddRef, Start or ShowEffect call.

Consequently, the foreign output write determines the retained identity. A
completed request that leaves H null permits another request on the next Init.
A request that writes E makes the next Init a no-op, regardless of its status
register. If the foreign request leaves the output untouched, H remains null.
The helper neither retries within one Init nor manufactures a fallback effect.
Factory absence is not a supported missing-reply result: the global and its
vtable are dereferenced without a guard. Factory failure/exception behavior is
not inferred from the helper's lack of status handling.

### 4.2 ShowEffect

Only the low byte of its four-byte stack argument selects the requested state
(`0x005FB2D0–0x005FB2D9`). H null means no foreign call. Otherwise it always
queries H's `+0x14` operation once, using only AL from that reply:

| Requested low byte | Active reply AL | Subsequent helper dispatch |
|---|---|---|
| Nonzero | Zero | Reload H and tail-dispatch Start(0) |
| Nonzero | Nonzero | None |
| Zero | Nonzero | Reload H and tail-dispatch Stop(0) |
| Zero | Zero | None |

For Start/Stop the helper rewrites the stack argument to the **32-bit value
zero**, then jumps through the foreign slot (`0x005FB2EA–0x005FB2F7`,
`0x005FB307–0x005FB314`). No helper field changes. Repeating the same requested
flag still queries the effect; deduplication is based on the fresh reply, not
the last requested flag. A primitive may report the old state again, in which
case the same command can be sent again. This is an active-state control
contract, not a guarantee that pixels are visible after Start.

### 4.3 Shutdown versus destruction

Shutdown first tests H. With null H it performs no operation. Otherwise it
queries active state once; when AL is nonzero it reloads H and calls Stop(0).
After the optional Stop it **reloads H and tests it again**, then saves that
pointer as the Release receiver, writes null to H, and tail-dispatches Release
(`0x005FB2A3–0x005FB2CA`). Clear-before-release is observable ordering: H is
already null when the Release primitive is entered. No direct AddRef or
deallocation is performed by this body. A full foreign Release implementation
is outside this spec.

On ordinary completed calls without callback mutation: inactive E gives
Query(E), clear H, Release(E); active E gives Query(E), Stop(E,0), clear H,
Release(E). Both leave H null. Another Shutdown then has no calls, as does the
destructor-labeled entry after that completed Shutdown. The explicit rereads
are native facts; they do not establish a general reentrancy contract.

The destructor-labeled body instead reads H, tests it, and when nonnull
tail-dispatches Release **without** querying activity, stopping the effect or
clearing H (`0x00456A20–0x00456A2B`). It supplies no deletion flag and performs
no allocator call. This tiny generic cleanup body is qualified as in §1.1;
there is no claim that it includes a deleting-destructor wrapper. Destruction
ends the object's lifetime; the retained old word is not reusable live state.
Repeated destruction is unsupported, not a second safe release or an expected
false result. Shutdown is therefore not interchangeable with destruction of
a still-populated helper.

### 4.4 Return, ownership and write ledger

| Operation | Owned persistent writes | Foreign effects | Result qualification |
|---|---|---|---|
| Constructor | H := null | None | 32-bit helper address in EAX |
| Init, H null | None directly after construction; foreign writes directly into H | One named creation request | Factory result untested; no helper-defined success/failure result |
| Init, H nonnull | None | None | No helper-defined result |
| ShowEffect | None | Guarded query, optional Start(0)/Stop(0) | No helper-defined result; a tail path can expose foreign registers |
| SetPosition | None | Five scalar requests, two floor calls, one SetTransform; none if H null | No helper-defined status; null-H path happens to leave EAX zero, nonnull path does not synthesize a bool |
| Shutdown | H := null before Release when reloaded H is nonnull | Guarded query, optional Stop(0), Release | No helper-defined result |
| Destructor-labeled body | None | Release if H nonnull | No helper-defined result; no post-lifetime H guarantee |

The field retains the factory-supplied reference. The helper has no independent
reference counter, pointer-equality setter or replacement API. Any reference
acquisition inside the named creation primitive is foreign, not a second
helper AddRef. No optional caller output, sentinel-write channel or error code
exists in the inspected public entries. Return-register leftovers must not
be promoted to an API bool; exact source-language return declarations remain
unproved where the export says `undefined` (§1.1).

## 5. Independent literal traces

All cases below are **PROPOSED / UNEXECUTED**. They were fixed analytically
before any future portable output and are neither native execution nor
rendering evidence. Each row supplies raw foreign replies, never a final
placement/whole-helper reply. The ordered events and semantic descriptor
fields are the expected observations; uninitialized padding is excluded.

### 5.1 Numeric environment and geometry literals

For these conditional traces use x87 CW=`0x037F` (PC=64-bit significand,
RC=nearest-even, masked exceptions), MXCSR=`0x1F80` and a nonzero floor-dispatch
word, so that the inspected finite SSE2 floor path is selected. Scalar replies
are the exact finite ST(0) dyadics listed; no primitive changes the control
environment. No claim is made that every original thread starts in that state
or that every scalar reply is realizable by an as-yet-unproved declared return
type (LGH1). All listed position inputs are exactly binary32-representable.

For each G row take H=E, with either active state, and supply its five scalar
replies in order. Expected calls are s1, Floor(Qx), s2, s3, Floor(Qz), s4, s5,
SetTransform(E,T), with no active query/Start/Stop. The identity 3×3 and first
byte 2 are common to every descriptor.

| ID | Position (x,y,z) | Raw replies (s1,s2,s3,s4,s5) | Floor inputs and replies, in order | Expected (Tx,Ty,Tz); stored scale; flag byte +1 |
|---|---|---|---|---|
| G1 | `(17.5,-3.25,-0.5)` | `(16,16,16,16,16)` | `35/32 → 1`; `−1/32 → −1` | `(17,-3.25,-15)`; 256; 3 |
| G2 | `(17.5,-3.25,-0.5)` | `(8,8,8,8,8)` | `35/16 → 2`; `−1/16 → −1` | `(16.5,-3.25,-7.5)`; 128; 3 |
| G3 | `(17.5,-3.25,-0.5)` | `(8,32,4,64,2)` | `35/16 → 2`; `−1/8 → −1` | `(18,-3.25,+0)`; 32; 3 |
| G4 | `(3/32,-3.25,-1/32)` | Five replies of `1/16` | `3/2 → 1`; `−1/2 → −1` | `(17/256,-3.25,-15/256)`; 1; 1 |
| G5 | `(+0,−0,+0)` | Five replies of 16 | `+0 → +0`; `+0 → +0` | `(1,−0,1)`; 256; 3 |
| G6 | `(17.5,-3.25,-0.5)` | `(16,16,16,16,3/32)` | Same as G1 | `(17,-3.25,-15)`; 3/2; 3 |
| G7 | `(17.5,-3.25,-0.5)` | `(16,16,16,16,1/16 + 2^−30)` | Same as G1 | `(17,-3.25,-15)`; **stored 1**, held `1 + 2^−26`; **3** |

G7 is specifically a raw-ST(0) conditional case, not an assertion that a
binary32-returning service could produce that reply. The held product is
nonunity; rounding its binary32 store to 1 does not clear flag bit 1.

Binary32 words for descriptor `(Tx,Ty,Tz,scale)`, written as hexadecimal word
values rather than host byte order:

| ID | Four expected words |
|---|---|
| G1 | `41880000 C0500000 C1700000 43800000` |
| G2 | `41840000 C0500000 C0F00000 43000000` |
| G3 | `41900000 C0500000 00000000 42000000` |
| G4 | `3D880000 C0500000 BD700000 3F800000` |
| G5 | `3F800000 80000000 3F800000 43800000` |
| G6 | `41880000 C0500000 C1700000 3FC00000` |
| G7 | `41880000 C0500000 C1700000 3F800000` |

For G1 the exact products are 16 and −16; adding 16/16 gives 17 and −15.
For G3 they are 16 and −4; the independent offsets are 32/16=2 and 64/16=4,
so translation becomes 18 and +0, while the fifth reply gives scale 32.
For G4, `1/16 + 1/256 = 17/256` and `−1/16 + 1/256 = −15/256`;
all stages are exact dyadic arithmetic. G5 preserves the middle negative-zero
word by integer copying. G7's excess `2^−26` is smaller than half a binary32
ulp above 1, which is `2^−24`; the stored word is therefore unity under the
stated rounding mode, while the x87 comparison is unequal.

Boundary family B: H=E, y=−3.25, z=−0.5, five replies of 16. All members
send a new descriptor, with Ty=−3.25, Tz=−15, scale=256, flag byte 3.

| x input | Stored floor input Qx | Floor reply | Tx |
|---|---|---|---|
| −16 | −1 | −1 | −15 |
| −1/4 | −1/64 | −1 | −15 |
| −0 (word `80000000`) | −0 | −0 | 1 |
| +0 | +0 | +0 | 1 |
| 63/4 | 63/64 | 0 | 1 |
| `16 − 2^−20` (word `417FFFFF`) | `1 − 2^−24` | 0 | 1 |
| 16 | 1 | 1 | 17 |

The negative noninteger distinguishes floor from truncation, and the last two
rows distinguish the exact boundary. No square-bound or orientation vector
is expected: this API has neither dimensions nor an orientation input.

### 5.2 State, identity and event traces

These are ordered primitive-call traces. Q(E)=the `+0x14` query; S(E,0),
K(E,0), R(E) denote Start, Stop, Release. C(name,&H) is the factory request.
Q replies are explicit inputs rather than inferred results of earlier commands.
No direct helper refcount increment/decrement or callback is invented.

| ID | Initial state / explicit replies and operation | Expected ordered observations / final state |
|---|---|---|
| L1 | Fresh storage; construct, then Init. C writes E and returns any status. | Construct: H=null, no calls, EAX=helper address. Init: C(`local_grid`,&H) once; H=E. No helper Start/AddRef/transform. |
| L2 | L1 completed; repeat Init. | No calls and H remains the identical E; no replacement request, even if a different resource would now be available. |
| L3 | Constructed H=null; Init; C returns failure status and leaves its output untouched. Repeat Init; C again leaves output untouched. | One C on each Init, H=null throughout. No status interpretation, fallback or extra retry inside either call. |
| L4 | H=null; Init; C writes E while returning a failure status. Then Init again. | First call retains E without testing status; second makes no call. This is a conditional raw foreign reply, not a promised factory behavior. |
| L5 | H=null; ShowEffect(1), ShowEffect(0), SetPosition, Shutdown; then destroy. | No primitive calls. SetPosition does not read its position or service; destructor finds null. All completed pre-destruction states retain H=null. |
| L6 | H=E; ShowEffect(1), Q reply=0; ShowEffect(1), Q reply=1. | Q(E), S(E,0); then Q(E) only. H remains E. |
| L7 | H=E; ShowEffect(0), Q reply=1; ShowEffect(0), Q reply=0. | Q(E), K(E,0); then Q(E) only. H remains E. |
| L8 | H=E; twice ShowEffect(1), both Q replies=0. | Q(E), S(E,0), Q(E), S(E,0). No last-flag cache suppresses the second Start. |
| L9 | H=E; ShowEffect stack word `00000100`, Q reply=1; then stack word `00000002`, Q reply=0. | First Q(E), K(E,0); second Q(E), S(E,0). Only the low request byte is tested; both command arguments are rewritten to full-word zero. |
| L10 | E with active query reply 0 supplied for a preceding hide; call G1 twice, then change x to 17.75, then x to 32, always with five replies of 16. | Hide: Q(E) only. Each move performs all eight G events. First three descriptors equal G1; fourth translation is `(33,-3.25,-15)`. No move queries visibility. |
| L11 | H=E; ShowEffect(1) with Q=0; then G1. | Q(E), S(E,0), then all eight G1 events and G1 descriptor. The shown move uses the same geometry as L10's hidden move. |
| L12 | H=E; Shutdown with Q=1; repeat Shutdown; then destroy. | Q(E), K(E,0), **H:=null**, R(E). No later calls. Release observes H already null. |
| L13 | H=E; Shutdown with Q=0. Then Init, with C writing distinct F. | Q(E), **H:=null**, R(E), then C(`local_grid`,&H); final H=F. New identity is acquired only after teardown, with no live-pointer replacement operation. |
| L14 | Live H=E, regardless of active state; invoke destructor-labeled body as destruction. | R(E) only; no Query/Stop/field-clear and no helper allocator call. No valid repeated-destructor case is promised. |

For L5 the unconsumed position can be represented by a harness sentinel; it is
not a supported invalid-position case on the nonnull path. There is no optional
output-sentinel test because the helper exposes no optional output argument.
Factory output H is the one real output-write channel, covered by L1–L4.
Null source/factory global, arbitrary factory exceptions, live-pointer
replacement and post-destruction reuse have no supported expected-false trace.

## 6. Explicit limits and questions

The unresolved environment/declaration boundary is tracked as LGH1 in
[questions/local-grid-helper.md](questions/local-grid-helper.md). Shared tiny
body attribution remains qualified in §1.1. Neither unknown is permission to
invent a helper bool return, a null-global fallback or constructor defaults
beyond the single null field.
