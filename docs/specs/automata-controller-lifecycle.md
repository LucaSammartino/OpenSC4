# Automata controller: inherited base lifecycle

## 1. Scope and evidence

This is a bounded Windows 1.1.641 contract for behavior owned by
`cSC4AutomataController`, supplementing [automata.md](automata.md) §3.5 and
[the controller-manager contract](automata-controller-manager.md). It is **partial**,
not a complete automata owner or an admission for a native wrapper. Generator,
attractor, Automaton and manager algorithms remain separate. Addresses below cite
the matching Windows export and instruction bytes; decompilation was navigation,
not an authority for numeric types or signatures. Proposed traces are unexecuted.

All ordinary call paths below assume valid storage, normal returned calls and no
reentrant mutation of controller fields or list links. No invalid-pointer guard,
exception recovery, allocation-failure unwind or foreign retention policy is
implied. A primitive whose return is ignored can return a failure value and the
controller still performs its next stated operation. Nonreturning calls are a
different domain.

## 2. Pointer views and state

Let **C** be the primary controller interface/object pointer. The serializable
interface is **S = C + 4**: constructor `00517D4E–00517D54` installs the two
vtables; QI `00514F80–00514F87` returns this adjusted pointer; secondary QI,
AddRef and Release thunks `00517F30`, `005183B0`, `005183C0` subtract four.
Write `00517FDC` uses its incoming S directly. Its saved position address S+0x10
(`00518060`) is C+0x14, matching GetPosition `00515A5E–00515A70`.
Saved flags S+0x3C (`0051808E`) are C+0x40; saved list head S+0x40
(`005180A4`) is C+0x44. These are two pointer views, not overlapping live fields.

| C offset | Width / role | Constructor writes (`00517D40–00517DE4`) |
|---|---|---|
| +0 / +4 | primary / serializable vtable pointer | `00A95D90` / `00A95D74` |
| +8 | initialized byte | 0 |
| +0x0C | retained occupant pointer | null |
| +0x10 | borrowed template pointer | null |
| +0x14, +0x18, +0x1C | position, three binary32 words | copies global `00B227CC`; value qualification below |
| +0x20 | unsigned 32-bit reference count | 0 |
| +0x24…+0x27 | four grid-bound bytes | each 0 |
| +0x28…+0x38 | embedded string/storage state | vtable `00A80810`; eight-byte allocation, empty terminator, begin=end and capacity=begin+8; +0x38 zero |
| +0x3C | selected sound ID, unsigned word (not a returned playback handle) | 0 |
| +0x40 | unsigned 32-bit flags | 2 |
| +0x44 | controlled-item list sentinel pointer | twelve-byte allocation; next and previous point to sentinel |
| +0x48 | unsigned 32-bit controlled-item count | 0 |
| +0x4C | retained clock/service pointer | null |
| +0x50 / +0x54 / +0x58 | binary32 elapsed lifetime / lifetime / activation delay elapsed | all positive-zero bits |

No constructor write establishes padding bytes +9…+0x0B or the sentinel payload.
The constructor is an internal ECX-receiver body returning C in EAX; this does
not establish a public constructor ABI or a callable standalone base factory.

## 3. References and interface outputs

Base QI (`00514F60–00514FB4`) recognizes IDs 1 and `6A1857F8`, storing C;
ID `E4FDA3D4` stores S. Each hit stores the output **before** calling virtual
AddRef and returns true in AL. A miss returns false in AL, leaves the output
untouched and does not acquire a reference. Both arguments are four-byte stack
slots, popped by `ret 8`. Null output storage is unchecked. The apparent
null-receiver arm for the adjusted hit still dereferences the receiver for
AddRef: it is not a supported null-object guard.

AddRef (`008AE800–008AE807`, shared body despite its export name) increments
C+0x20 modulo 2^32 and returns the stored count in EAX. Release
(`00517F10–00517F27`) unsigned-compares the count with 1: above 1 it decrements,
stores and returns the count; at 0 **or** 1 it calls virtual slot 39 (+0x9C)
without first decrementing/storing the count, then returns zero in EAX.
No atomic operation or underflow guard is present. The slot-39 body at
`00519340` is mislabeled Write in the export: it calls virtual Shutdown, then
the base destructor, then heap free. Thus final Release cleanup is different
from direct entry to the base destructor.

## 4. Template and occupant assignment

SetTemplate (`00517430–0051745D`, slot 15) first stores the raw pointer at
C+0x10. A nonnull template clears the embedded string only if nonempty, copies
template+0x4C to lifetime C+0x54 and template+0x58 to flags C+0x40. A null
template leaves string, lifetime and flags unchanged. Same-pointer assignment
still repeats the nonnull effects. It neither AddRefs the new template nor
Releases the old one, and does not reset either elapsed field or activate.
It pops one pointer argument and defines no boolean result.

SetOccupant (`00517F40–00517F6E`, slot 14) compares pointer identity. For a
different value: AddRef new if nonnull; store new at C+0x0C; Release old if
nonnull. Same-pointer assignment performs none of these. Returned AddRef/Release
values are ignored. Null detaches by storing null before releasing old. No
position copy, grid update or controlled-item removal is part of this body.
It pops one pointer argument and defines no boolean result.

## 5. Shutdown and destruction

Shutdown (`005169F0–00516A90`, slot 4) always calls virtual Activate(false)
first. Only then it tests initialized byte C+8. If clear, it returns true in AL
without clearing template, occupant, clock or list. If set, it clears that byte
**before** walking the controlled-item list from sentinel.next in link order.
For each node it saves payload A from the pointer word at node+8, calls virtual CheckFlag(0x100), and if
true calls A slot 13 (+0x34) with null; it then calls A Release. Both returned
values are ignored. The next link is read after these calls. This specifies
controller-owned ordering, not the transitive automaton detachment policy.

After all payload calls, a separate pass frees each node, restores sentinel
self-links and stores count zero. Then it saves occupant, stores null before
its Release; saves clock, stores null before its Release; and finally stores
template null. It returns true in AL on every normal returned path. No template
Release occurs. Citations: payload pass `00516A13–00516A3C`; node pass and count
`00516A3F–00516A67`; references/template `00516A6A–00516A89`.

A second Shutdown still calls Activate(false), but the initialized-byte test
skips the cleanup pass. Under inherited base Activate and unchanged state this
does no further cleanup. A reference attached after Shutdown can therefore
survive repeated Shutdown until destruction or reassignment. This is conditional
idempotence, not a general guarantee for overridden/reentrant calls.

The direct base destructor (`00517F80–00517FC8`) reinstalls base vtables,
Releases a nonnull clock, destroys list **storage**, destroys string storage,
then Releases nonnull occupant. The list helper `005650A0–005650DD` frees nodes
and sentinel without calling payload Release or detach. The base destructor
does not call Shutdown, clear initialized, release template or release each
controlled payload. It does not null the clock/occupant fields before calling
their Releases. Do not substitute a modern automatic ownership cleanup model.

## 6. Initialization and activation

Init (`00516850–005169E5`, no stack arguments, true in AL) returns immediately
when C+8 is nonzero. Otherwise it stores 1 there, zeros all four grid bytes,
then calls city service +0xC0 to obtain K. If K differs from C+0x4C it AddRefs
nonnull K, stores K, then Releases the old nonnull pointer. Equal pointers
cause neither reference call. A null returned K is permitted here and replaces
an old reference; there is no rollback or false return.

Next it calls virtual CheckFlag(0x20). When false, it leaves position unchanged.
When true it obtains the lot service from city +0x84; with nonnull service and
occupant it asks service+0x38 for that occupant's lot. A nonnull lot's +0x38
fills four signed 32-bit cell bounds (x0,z0,x1,z1). The controller computes
x = 16*x0 + 0.5*(16*(x1-x0+1)) and the analogous z. Subtraction, increment and
left shifts occur in 32-bit registers, then signed integer-to-x87 conversion;
the stored x/z are binary32. This formula is supported for finite bounds without
32-bit overflow, subject to ambient x87 precision/rounding. Do not reinterpret
it as an unbounded integer formula at overflow (`005168F7–00516950`).

The city +0x188 predicate receives x,z. A nonzero AL obtains y from terrain
service `00B43CF4` +0x34(x,z), returned in ST0 and stored binary32. A zero AL
uses occupant+0x20(out-vector), taking only its y; if occupant is null it stores
y=0. If no lot is found, nonnull occupant+0x20 writes directly into C+0x14;
without occupant position stays unchanged. Null service/lot and false predicate
are ordinary returned alternatives, not Init failures. Output-producing calls'
status values are ignored: callers must provide their actual output mutations,
including unchanged words when that is what a primitive does.

Finally Init calls virtual Activate(true), virtual CheckEffectTriggers, and
owned PlaySFX(true), in that order, then returns true. Initialized was already
stored before all service calls. No cleanup rollback is visible if a call does
not return. Sections 6.1–6.3 specify the selected inherited sound/view/effect
helpers. Virtual overrides and unsupported domains remain separate; no owned
helper may be replaced with a supplied whole-controller result.

Base Activate (`00516AA0–00516B33`, one four-byte slot whose low byte is used,
no defined result) does nothing when uninitialized. Otherwise it compares the
AL result of CheckFlag(1) with the **raw low byte** of the request. Equality is
a no-op. On inequality: call SetFlag(1,request), obtain the controller manager
via city+0xBC, and for nonzero request call manager+0x40(C). Both directions
then call owned PlaySFX(request), virtual HandleViewChanged(), store delay
elapsed C+0x58 = +0, and call SetFlag(0x400,false). A nonzero request with a
nonnull template whose binary32 +0x50 is ordered-greater-than zero additionally
calls SetFlag(0x400,true). There is no manager remove call on this base path.
All returned values from these operations are ignored. Manager may be null
without an immediate dereference on the false direction, but the true direction
dereferences it unchecked. The inherited sound/view expansion follows below.

CheckFlag (`00517E60`) returns EAX=0 or 1 for any overlapping mask bit, not
an all-bits test. SetFlag (`00517E70`) sets masked bits only when the input
low byte is exactly 1; every other byte clears them. Thus Activate byte 2
is not a synonym for true: it can clear bit 1 while taking nonzero-request
notification/delay branches. This raw-input discriminator must be preserved.

IsActive (`00517E20`) short-circuits virtual CheckFlag calls for masks 1,2,4,
0x400 in that order. It returns true in AL iff the first three are nonzero and
the fourth zero. IsTickable (`00518330`) forwards CheckFlag(1), independently
of masks 2,4 and delay. NotifySourceDataChanged (`00514E30`) with a nonnull
template calls SetFlag(template+0x58,true), then SetFlag(2,true), copies
template+0x4C to lifetime and zeros delay elapsed. It does not replace the
entire flags word, reset lifetime elapsed, or activate; null template does nothing.

### 6.1. Owned PlaySFX: selection, start, position and stop

Primary vtable `00A95D90` resolves slot 25 to `00516B40` and slot 27 to
`00515870`, both with C as receiver; there is no S adjustment on these calls.
PlaySFX is a direct ECX=C call, with one four-byte stack slot whose low byte
is used, `ret 4`, and no defined result. Its two exported direct callers are
Init `005169D9` and Activate `00516AE5`. The following is its owned CFG, not
the implementation of the sound service.

PlaySFX snapshots sound service D from `00B43CB8`. Null D returns immediately,
preserving selected ID C+0x3C. For request byte zero, a zero selected ID also
returns; otherwise it calls D+0x2C with the five words
`(6A451908,C,0,0,0)`, ignores the return, and returns. It does **not** clear
the selected ID or release anything (`00515846–00515865`). Thus repeated direct
stops can repeat the same command; selection ID zero suppresses a later stop
even if an earlier start operation returned success.

For any nonzero request byte (including 2), null template or equal template
begin/end pointers at +0x60/+0x64 returns without changing the selected ID.
Otherwise construct a local random object with seed word FFFFFFFF; call its
uniform-unsigned operation with `(end-begin) arithmetic-shift-right 2`; take
returned index i and store the word at template.begin[i] to C+0x3C **before**
obtaining position or issuing sound commands (`00515791–005157D8`). The domain
here is a valid contiguous four-byte-element range, positive representable
count and in-range returned index. Malformed ranges/indices, random internals
and allocation/exception effects are not normalized to failure.

Initialize a temporary vec3 with three copies of the raw word at `00B227CC`,
then virtual GetPosition(out). Its output mutation, rather than its ignored
return, supplies coordinates; inherited GetPosition is §8. Call D+0x2C with
`(EA451903,selectedID,C,0,0)`. Then convert z, y, x, in that order, through
CRT `_ftol2` (`009EEF04`), taking each EAX low word; call D+0x2C with
`(CA45190F,C,ix,iy,iz)`. The second command is issued even if the first returned
zero/false, and the selected ID is not replaced with either return value.
Stack-relative loads `00515812–00515831` establish z/y/x conversion and x/y/z
argument order. This contract uses exact small integer coordinates for literal
examples; invalid conversions, traps and values outside signed 32-bit range
are unsupported here. No sound AddRef/Release or playback-handle ownership is
established. No initialization, active, suppression, visibility or delay flag
test exists inside PlaySFX.

### 6.2. Owned CheckEffectTriggers: ordered predicates and source Release

Base slot 25 has no explicit arguments, no stack cleanup and no defined result.
Null template returns immediately. Otherwise snapshot R=`00B43DC8`; null R
returns. Zero a local output word Q; call R.QI(`FA47B3F9`,&Q), testing only AL
(`00516B46–00516B7E`). False AL returns immediately: even a nonnull written Q
is neither used nor released. True AL proceeds without null-checking Q.

Template+0x44 and +0x48 are pointers to circular sentinel lists, next at node+0,
predicate ID at node+8. Start predicate p=1. Visit the first list in order while
p is nonzero: call Q+0xB8(ID), replace p by 0/1 according to nonzero AL, then
read the node's next link. For the second list, continue only while p is nonzero;
each Q+0xB8(ID) instead sets p to 1 iff AL was zero. Stop after the first failed
condition, with no later predicate calls. Therefore the condition is all positive
IDs true AND all negative IDs false; positive failure prevents all negative
queries. Repeated IDs remain repeated calls (`00516B86–00516C0A`).

If either list was nonempty, virtual Activate(p) follows; if both are empty,
there is no Activate call (not Activate(true)). Only the argument's low byte is
defined as p; upper bytes are scratch. Finally call **R.Release**, not Q.Release,
and ignore its result (`00516C10–00516C37`). Q is the receiver of predicate
calls, R is the receiver of the final Release even when these pointer identities
differ. This asymmetry is an observed call rule, not a claim about the foreign
QI's retention convention or a repair to it. True-with-null Q is a defined
normal path only with both lists empty (no Q dereference); with a nonempty
list it reaches an unchecked null dereference, outside the supported domain.
There is no extra AddRef or local cleanup on false QI, nor a rollback of a
completed Activate if the final Release returns a failure-like value.

### 6.3. Owned HandleViewChanged: ordered point tests and visibility store

Base slot 27 takes no arguments, returns true in AL on every normal path,
and pops no arguments. Snapshot V=`00B43DD8`; if V or terrain global `00B43CF4`
is null, return true with **no flag update**. Otherwise CheckFlag(0x200);
nonzero forces SetFlag(4,1) and returns true, skipping bounds, position, city,
terrain and frustum calls (`00515870–005158AC`, `00515A13–00515A35`).

For zero flag 0x200, call virtual GetBoundingBox(&A,&B). A and B are two local
vec3 outputs; their initial words are not established here. The inherited
bounds computation is §8, not a foreign whole-bounds oracle. Its returned
status is ignored. Initialize local P with three copies of raw `00B227CC`,
then GetPosition(&P). Snapshot city J=`00B43CEC` (unchecked). Query J+0x188
with A.x,A.z; nonzero AL replaces A.y with binary32 terrain+0x34(A.x,A.z),
zero copies P.y. Do the same for B.x,B.z and B.y. These queries happen even
though inherited GetBoundingBox already sampled heights; they do not update
C's stored position (`005158AC–0051595D`).

Initialize another temporary P2 from the same raw global, call GetPosition(&P2)
again, and call `007F7E50` with ECX=V and pointer to P2. This single foreign
frustum predicate returns AL. If false, test the following points in order,
stopping at the first nonzero AL: A; (B.x,A.y,A.z); B; (A.x,B.y,B.z).
These are constructed by local raw word copies, not vector interpolation
(`0051595D–00515A11`). There are at most five frustum calls, and the second
GetPosition is not reused from P (a foreign occupant may produce different
positions on separate calls). SetFlag(4,1) on any true predicate, else
SetFlag(4,0), then return true regardless of visibility or SetFlag's return.
Only the setter's low-byte argument is initialized; higher bytes are scratch.
No controller list, sound ID or reference count is modified by this helper's
own stores. Returned false predicates are ordinary alternatives, not a false
HandleViewChanged result. Bounds output not written before use, invalid service
pointers, reentry and traps remain unsupported, with no invented fallback.

## 7. Animation time and expiry

DoAnimationTick (`00514D90–00514E29`, slot 38) pops **12 bytes**. It reads only
the first four-byte argument as binary32; meanings/types of the other two words
are unresolved. It multiplies the first argument by the binary32 constant
`0x3A83126F` (0.0010000000474974513), then stores binary32 delta before calling
CheckFlag(1). All following accumulation/comparison is x87; stores are binary32.
For finite values and ordinary masked FP operation:

1. If CheckFlag(1) is zero, no elapsed fields change.
2. If CheckFlag(0x400) is nonzero, add delta to delay elapsed and store it.
   With a nonnull template, compare the still-live x87 sum with template+0x50;
   at or above the delay, call SetFlag(0x400,false). A null template leaves
   the flag set. (`00514DBC–00514DE7`.)
3. Test CheckFlag(0x400) again. If now zero and lifetime is ordered >0, add
   the **entire** delta to lifetime elapsed, store it, then compare the live
   sum with lifetime. Only a strictly greater sum calls Activate(false).
   Equality does not deactivate (`00514DE9–00514E23`). Crossing delay does
   not subtract the delay portion of this tick.

SetLifetime (`00517EF0`, one four-byte argument) copies its bits to C+0x54
without a reset or validity test. IsExpired (`00517EA0`) returns EAX=1 iff
lifetime is ordered >0 and stored elapsed is ordered >= lifetime; otherwise 0.
Consequently equality can be expired while still activated. Activation and
Shutdown do not reset lifetime elapsed; a fresh Init does not reset it either.
Positive-zero constructor state is the only reset established here.

There is no integer tick counter or integer time-wrap subtraction in these
bodies. Negative deltas, infinities, NaNs, FP traps, x87 control changes and
overflowing accumulation are outside the finite numeric contract; do not
invent wrap-to-zero behavior. The delay and lifetime comparisons can use an
x87 intermediate more precise than the simultaneously stored float, so portable
round-then-compare equivalence is not established at every rounding boundary.
Binary constants were read from the matching PE image: SHA256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`;
`00A84D2C` is 0.5 and `00A81054` is +0. Global `00B227CC` is in virtual
zero-fill storage; its runtime initialization/value was not traced. Constructor
position therefore remains a copied raw word, rather than an assumed runtime zero.

## 8. Position, world bounds and grid bytes

Slot 17 (`00602F90`, misleading shared symbol SetKey) takes a pointer to three
words, copies them in order to C+0x14,+0x18,+0x1C and pops four bytes. It has
no boolean result, reference call, occupant move or manager notification.
GetPosition (`00515A40`) when occupant is nonnull and CheckFlag(0x20) is false
tail-calls occupant+0x20 with the original output pointer. Otherwise it copies
the three stored words to output. The inherited local copy returns the output
pointer in EAX incidentally; do not require a boolean success return. The
occupant path forwards the foreign call's return channel. Template presence
is **not** this accessor's branch predicate.

GetOccupantLot (`00515D20`) returns a borrowed lot pointer in EAX: get city+0x84,
then with nonnull service and occupant call service+0x38(occupant), else null.
It performs no AddRef. GetOccupantBoundingBox (`00516EC0`) takes two vector
output pointers, pops eight bytes and defines no boolean result. If either
CheckFlag(0x20) or CheckFlag(0x40) is true, call virtual GetPosition separately
for min and max. With neither bit and null occupant, do the same. Otherwise
call occupant+0x28(min,max). Then, if GetOccupantLot returns nonnull, ask its
+0x38 for signed cell bounds and overwrite only x,z with 16*x0,16*z0 and
16*(x1+1),16*(z1+1); preserve the occupant-written y words. The integer shifts
precede signed conversion; overflow remains unsupported. (`00516F15–00516F5A`.)

GetBoundingBox (`00515BD0–00515D12`, slot 21, two output pointers, ret 8,
no boolean result) first calls virtual GetOccupantBoundingBox(min,max), then
virtual GetRadius() in ST0. It subtracts radius from min.x/min.z and adds
radius to max.x/max.z, storing each as binary32. It gets x extent via
city+0x164 then z extent via +0x168, storing each binary32. For ordered finite
coordinates and finite nonnegative extents it clamps each x/z independently
to [0,corresponding extent], in order min.x,min.z,max.x,max.z. It does not
sort reversed endpoints. Finally it gets a height service via city+0x150 and
calls service+0x34(min.x,min.z), storing min.y; then the same for max, storing
max.y. Thus y is resampled at two corners, not taken as a vertical min/max of
all terrain. All required service pointers are unchecked. Negative radius,
nonfinite values and foreign height semantics are not normalized by this spec.

Base GetRadius (`00517E00`) returns template+0x20 binary32 in ST0, or +0 with
null template. GetTemplateName (`00517DF0`) returns the address of the template's
name object at template+4, else the controller's embedded name object C+0x28;
it does not return a newly owned string. Slots 6/8 (`00517F70` / `0040CE60`)
return the raw occupant/template pointers in EAX without acquiring them.

Slot 34 (`00477810`, misleading shared name GetAreaToDrawTo) returns C+0x24
in EAX with no arguments. This is mutable storage for four grid-bound bytes;
constructor and first Init clear them. Neither SetPosition nor SetOccupant
computes membership. [automata-support.md §2](automata-support.md#2-the-controller-grid-csc4controllergrid-compat)
describes the separate grid owner; its membership/clamping algorithm is not a
controller primitive or additional owned coverage here.

## 9. Controlled-item removal and primitive boundaries

HandleRemoveAutomaton (`0051BD50–0051BDEE`, slot 31, one automaton pointer,
ret 4, true in AL on normal return) obtains the manager via city+0xBC. Its
first operation is an **inline** lookup of the current occupant key in the
manager's queue table, not a foreign method computing removal policy. It
unsigned-divides the occupant pointer bits by the bucket count derived from
(manager+0x74 minus manager+0x70) arithmetic-shifted-right two; traverses the
bucket chain comparing exact keys; on a match calls the single queue primitive
RemoveFromQueue(entry+8,A), ignoring its returned value. A zero bucket count,
null manager or malformed chain is unchecked, not an empty-table success case.
The queue primitive's internal release effects belong to the manager contract.

Then it scans the controller's own list in order for the first payload identical
to A. On match it calls A.Release **before** unlinking; patches previous.next
and next.previous, frees that node and decrements C+0x48 modulo 2^32. It stops
after one match, so duplicates are not all removed. No match leaves list/count
unchanged. This base removal body never calls A+0x34(null), even with flag 0x100.
The queue attempt occurs even if A is absent from the controlled list. Returns
from queue removal and A.Release do not control the true AL result. Exceptions,
callbacks that invalidate the node, and arbitrary malformed list/count pairs
are outside this normal-return contract.

IsFull (`00517ED0`) returns EAX=0 without a template or when template+0x54
capacity is zero; otherwise EAX is 1 iff the unsigned count C+0x48 >= unsigned
capacity. Count is not a float and is not recomputed in this base accessor.

Reusable foreign boundaries are **one call and its raw channels**:

| Primitive receiver / offset | Inputs and outputs used here | Controller's treatment |
|---|---|---|
| any retained interface +4 / +8 | AddRef / Release, EAX word | ignore result except controller's own ref API |
| city +0xC0, +0x84, +0xBC, +0x150 | no explicit args; EAX pointer | clock is retained; other pointers borrowed/unchecked or null-tested as above |
| city +0x188 | two binary32 words x,z; AL predicate | selects terrain y versus occupant y |
| city +0x164 / +0x168 | ST0 extent | store binary32 before clamp |
| lot service +0x38 | occupant pointer; EAX lot | null is a returned alternative |
| lot +0x38 | output pointer to four signed words | raw output mutation; no status branch |
| occupant +0x20 / +0x28 | vector output / two vector outputs | raw output mutation; no success policy supplied |
| height service +0x34 | x,z binary32 words; ST0 y | store binary32; no fallback policy |
| automaton +0x34 | null controller pointer | return ignored; transitive automaton effects foreign |
| controller manager +0x40 | C pointer | activation notification only; result ignored |
| random constructor `00913A5C` / uniform `009136B1` | local receiver, seed FFFFFFFF / positive count; EAX index | range/index domain §6.1; internals foreign |
| sound D+0x2C | five words, command layouts §6.1; raw return ignored | no returned handle stored; no start-failure rollback |
| effect source R+0 / Q+0xB8 / R+8 | IID and pointer-output / predicate ID / no args | QI AL gates traversal; predicate AL normalized; source Release result ignored |
| view predicate `007F7E50` | ECX=V, pointer to vec3; AL predicate | one point test; short-circuit/store logic is controller-owned |
| queue RemoveFromQueue `005172D0` | queue entry and A | one queue operation; result ignored, transitive ownership not closed here |
| heap allocation / free | byte count / exact allocation pointer | storage primitive only; no controller policy |

Owned virtual calls and PlaySFX/View/Effect helpers are composition points, **not**
foreign whole-result hooks. Clock advancement, template lifetime management,
manager membership policy, queue side effects and automaton state changes cannot
be smuggled into one host callback that supplies a completed Shutdown result.

## 10. Concrete dispatch and factory boundary

The automata director's registration sequence `00514A00–00514C1B` registers
attractor `6A1628BA` at `00514BE5–00514BF1` and generator `4A1628C5` at
`00514BF6–00514C02`; it registers no separate base-controller factory. These
factory bodies (`00514860`, `00514880`) allocate 0x6C / 0xA0 bytes, null-check
allocation and call derived constructors `005181F0` / `005182B0`. Both call
base constructor `00517D40` with the same primary receiver before installing
derived tables. The base's GetGZCLSID (`00517F00`) really returns `4A18672B`:
this is a **serializable identity**, not evidence of an executable base factory.
Only two direct callers of the base constructor appear in this export, the
two derived constructors. These are bounded director/export facts, not proof
against every possible unexported or indirect external creation route.

| Slot / role | Attractor table `00A95E48` | Generator table `00A95F00` |
|---|---|---|
| 0–2 QI/ref | shared QI jump `005151F0` to base; base ref bodies | same |
| 3 Init | override `00516FB0`, first-init prework then tail-enter base `00516FCE` | override `00516FE0` calls base when not initialized |
| 4 Shutdown | base | override `00517000`: only calls base if initialized; always true AL |
| 5 type / 16 groups | concrete overrides (base purecalls) | concrete overrides (base purecalls) |
| 9 radius / 15 template / 18 activation | base | overrides `00518310` / `00517460` / `00517240` |
| 24 time of day | inherited `006F7DC0` (ret 8, no defined result) | override `00515200` |
| 29/30 state enter/exit | overrides `0051BFB0` / `0051C030` | base |
| 35 expiry / 38 animation tick | base | overrides `00518350` / `00515E60` |
| 39 deleting cleanup | `005192C0`, virtual Shutdown then derived destruction/free | `00519340`, virtual Shutdown then base destruction/free |

Other shared slots in the base ledger below remain inherited on both routes.
Generator Activate reaches base Activate at `005172C1`, **after** derived
prework; do not apply the base-only list chronology to the entire generator
call. Generator SetTemplate is a distinct body, not an assumed base call.
No derived selection/emission/placement or motion algorithm is specified here.

## 11. Proposed literal discriminators — UNEXECUTED

These traces were derived from branch/call/store instructions before any portable
outputs. They are proposed conformance inputs, not measured native outputs.
Identity names C,K,O1,O2,A,B,T1,T2 denote distinct valid objects. All primitive
calls return normally without reentrant mutation unless stated. A lifecycle
trace containing an unresolved owned helper or unspecified helper inputs is a **projection**:
its call order is specified, but its full transitive effects are not predicted.

| Trace | Literal state / inputs | Required observable effects or boundary |
|---|---|---|
| L1 construction, initialization, repeat cleanup | construct C; assign O1 and T1; first Init returns city clock K; choose flags without 0x20; owned callbacks return normally; list initially empty; Shutdown twice | construction count 0, flags 2; assign AddRef(O1), no template ref; Init sets initialized, clears grid, AddRef(K), stores K, calls Activate(1), Effect, PlaySFX(1); first Shutdown calls Activate(0), clears initialized, resets list/count, nulls+Releases O1 then K, clears template; second still calls Activate(0), skips cleanup. Projection leaves sound/view/effect inputs unspecified; N1/N11 instantiate those paths. |
| L2 direct destruction versus final Release | initialized=1; clock K; occupant O1; controlled list [A,B]; then direct base destructor versus Release with count=1 and base slot39 | direct destructor: Release(K), free A/B nodes and sentinel, free string, Release(O1); no A/B payload Release. Final Release calls virtual Shutdown first and then destructor/free. With inherited base activation, Shutdown's payload Releases precede occupant then clock; destructor sees null service refs. |
| L3 two templates, identity and null | local name nonempty; elapsed=3; T1 lifetime=8, flags=0x107; T2 lifetime=13, flags=2; SetTemplate(T1), same T1, T2, null | each nonnull assignment clears nonempty name, copies lifetime/flags; same T1 still copies; null only stores null. Final lifetime=13, flags=2, elapsed=3; zero template AddRefs/Releases throughout. |
| L4 occupant replacement | O1 retained; SetOccupant(O2), same O2, null | AddRef(O2), store O2, Release(O1); identity no calls; store null, Release(O2). Repeat with AddRef returning 0 and Release returning 99: same sequence and stores. |
| L5 shutdown list chronology | initialized=1; base Activate request already false; flags=0x100; controlled list [A,B]; occupant O1, clock K; payload detach returns false, payload Release returns 17 | CheckFlag(0x100), A.detach(null), A.Release; then flag check, B.detach(null), B.Release; only afterwards free both nodes, count=0, null+Release O1, null+Release K, template=null; true AL regardless of those returned values. Clear flag case omits detach only. |
| L6 removal identity and duplicates | valid nonempty manager bucket table, queue lookup miss; list [A,B,A], count=3; remove A twice, then absent A | first Release(A) precedes unlink/free of first node, count=2 and list [B,A]; second Release(A), count=1, list [B]; third no payload Release. All three return true AL; queue lookup precedes list search. Queue hit adds RemoveFromQueue before each search even for absent A. |
| L7 raw boolean | initialized=1; flags=1; Activate byte=2; no overriding SetFlag; callbacks return | comparison 1 != 2; SetFlag(1,2) clears bit; nonzero request still invokes manager notification and can enable delay. A byte-1 request with flags bit already set is a no-op. Projection leaves owned-helper inputs unspecified; N6 supplies a concrete fixture. |
| L8 equality / wrap discriminator | flag1 set, delay clear; stored elapsed=1, lifetime=1; Tick first arg=0, other words arbitrary | sum stays 1; IsExpired returns EAX=1; tick does not call Activate(false). At elapsed=0x4F800000 (binary32 2^32), same lifetime, delta=0: same equality result, with no unsigned wrap. Lifetime=0 makes IsExpired false and tick skips lifetime addition. |
| L9 delay crossing | flags=0x401; delay elapsed=0; template delay=0.5; lifetime elapsed=0; lifetime=2; first argument=1000 | under x87 PC=53, round-to-nearest, multiply/store delta gives binary32 1; delay clears and full delta is also added to lifetime, yielding stored 1. Delay is not subtracted. Other PC/RC settings need independent qualification. |
| L10 asymmetric bounds | flags 0; nonnull occupant+0x28 writes min=(-2,7,30), max=(9,8,80), even with AL=false; city+0x84 returns null lot service; template radius=4; extents x=20,z=64; height replies 11 then 22 | owned GetOccupantBoundingBox calls occupant then owned GetOccupantLot (00515D20); null lot service preserves those outputs. Owned GetBoundingBox expands/clamps to min.xz=(0,26), max.xz=(13,64); call height(0,26) then height(13,64); final min=(0,11,26), max=(13,22,64). No supplied whole-controller bounds result. |
| L11 pointer-adjusted QI | count=0; output initially DEADBEEF; call via S with ID E4FDA3D4; then unknown ID | thunk subtracts 4; hit stores S, AddRef makes count1, AL=true; miss leaves output S and count1, AL=false. Repeat direct primary IID1 gives C and another acquired ref. |
| L12 position / grid separation | stored pos=(5,6,9); occupant reports (10,11,12); flags=0; SetPosition((1,2,3)), GetPosition(out), then set mask0x20 and get again | setter writes stored (1,2,3) only; first getter forwards occupant output (10,11,12); second copies (1,2,3). Neither setter/getter computes tracts; slot34 returns exact address C+0x24. |
| L13 inherited versus override | initialized=0; call Shutdown on attractor versus generator | attractor enters base and calls Activate(false) before the byte test; generator override returns true without calling base. Both constructor paths enter the same C-receiver base initializer; their virtual Init/Activate paths are not interchangeable. |

### 11.1. Independent literal helper composition ledger — UNEXECUTED

L1 and L7 above retain their original **projection** status: their unspecified
template, service and helper outputs do not define complete traces. The following
new fixtures supply single foreign-call replies, not whole-helper results.
C, S=C+4, occupant O, templates T/U, clock K0/K1/K2, city J, manager M, sound D,
effect source R, QI output Q, view V and terrain H are distinct valid identities
except where a row explicitly says null/equal. Template/list storage and object
lifetimes remain valid throughout returned calls. All dispatch is inherited base;
no generator/attractor whole-call chronology is inferred. These are source-derived
proposals with no native Init, playback, rendering, objects or game execution.

Unless overridden, C is initialized=0, flags=0x202, stored position=(12,5,28),
elapsed lifetime=3, lifetime limit=8, delay elapsed=9, selected sound ID3333,
empty controlled list, occupant O already
retained and old clock K0 retained. T is borrowed, radius=4, lifetime=8,
delay=0.5, sound words [1111,2222] (hexadecimal IDs), both trigger lists empty.
J+0xC0 returns K1, J+0xBC returns M. Each O+0x20 output is (12,5,28), even if
AL=false. RNG replies 0 then 1 on successive selections. D/R/V/H globals are
nonnull; R.QI returns AL=1 and Q. D+0x2C, M+0x40, AddRef and Release may return
zero: those channels do not alter the stated stores or next calls. Reference
effects **inside** each foreign operation remain that operation's outputs; no
unobserved QI acquisition/manager retention/sound handle is asserted. All floats
in these rows are binary32. Numeric predictions use masked exceptions,
round-to-nearest-even and x87 PC=53 unless a row states otherwise, not a measured
runtime control word. Small integer coordinate copies/conversions and exact
additions here also agree at PC=24/64 under that rounding mode.

| Trace | Change / action | Literal required sequence and final state |
|---|---|---|
| N1 first Init, empty effects | common fixture; Init | store initialized=1, zero grid; K1.AddRef while old field=K0, store K1, K0.Release; no lot calls (flag0x20 clear). Activate(1): flags203, get M, M+0x40(C), select1111, store ID, O.position, D start(EA451903,1111,C,0,0), then position(CA45190F,C,12,5,28); forced-view sets flag4, delay=0, clear/set400 gives flags607. Effect QI sees empty lists: no predicates/Activate, R.Release. Final explicit PlaySFX(1) selects2222 and repeats start/position. Init AL=true, flags607, ID2222, lifetime elapsed3; O/T unchanged. |
| N2 Init, effect disables | common fixture; T positive IDs [P1,P2], negative [N1]; Q+0xB8(P1) returns AL=0 | first Activate/start1111 as N1; QI then P1 only; Activate(0) clears bit1, gets M but no M notification, sends D stop(6A451908,C,0,0,0), forced-view, delay0/clear400; R.Release; Init still performs explicit PlaySFX(1), selecting2222 and sending start/position. Final flags206, ID2222, inactive despite final sound start; Init AL=true. No P2/N1 query, Q.Release or rollback. |
| N3 positive then negative short-circuit | initialized1, flags206; T positive [P1,P2], negative [N1,N2]; replies AL=2,7,0,3 respectively; call Effect | query P1,P2,N1,N2 in order; p=1,1,1,0. Calls Activate(0), which is already equal to flag1=0: no sound/view/delay/manager operation. Then R.Release. Final flags206, delay unchanged. Swap P2 reply to0: query only P1,P2, same p0. |
| N4 all conditions satisfied | initialized1, flags206; positive [P1], negative [N1,N2]; replies 2,0,0 | canonical p1; Activate(1) notifies M, selects1111, sends start/position, forced-view then delay0/clear/set400; final flags607; then R.Release, never Q.Release. With both lists empty, omit Activate entirely, preserving flags206 and delay9. |
| N5 raw QI output alternatives | call Effect with nonempty positive list; QI AL=0 but writes Q | no predicate, Activate, R.Release or Q.Release; fields unchanged. AL=1 with Q=null and both lists empty: no Q dereference/Activate, R.Release still occurs. AL=1/Q=null with nonempty lists is unsupported unchecked dereference, not a false Effect result. |
| N6 raw activation bytes | initialized1, flags207, delay9; Activate(2) | comparison1!=2; exact-one setter clears bit1 to206, yet manager notification and sound start1111/position occur; forced-view, delay0, clear/set400 =>606. Repeating byte2 repeats all this (flag1 remains0). Byte1 from flags207 is a complete no-op with delay9; byte0 from flags206 is likewise a no-op. Byte0 from flags207 sends stop iff ID!=0, no manager notification, view store and delay clear =>206. |
| N7 sound early exits / ID retention | direct PlaySFX(1), old ID3333; D=null, or T=null, or T begin=end | each alternative preserves3333 and makes no sound/position/RNG call. Restore D and call PlaySFX(0): one stop, ID still3333. Repeat direct stop: same stop again. Nonzero2 with valid range selects1111/start/position. A selected sound word0 still causes start(…,0,C,0,0) and position; following stop is omitted. |
| N8 unavailable/forced view | direct View, flags202, V=null or H=null | AL=true, flags202 unchanged; no setter or bounds/frustum calls. Both nonnull and flag200 set: SetFlag(4,1), flags206, AL=true; no bounds, position, city, terrain or frustum call. No flag1/2/400 gate. |
| N9 complete finite view path | initialized1, flags40, T radius4, stored pos(12,5,28), nonnull V/H; inherited bounds position path; J extents64,64; city height service gives11,22; later J+188 replies false then true; H+34 gives33; all five frustum replies false | owned occupant bounds calls GetPosition twice (flag40); each forwards O.position because flag20 is clear. World bounds A=(8,11,24), B=(16,22,32). First view GetPosition P=(12,5,28); city tests (8,24),(16,32) give A.y=5 and B.y=33 (one H call only). Second GetPosition P2=(12,5,28); frustum points exactly (12,5,28),(8,5,24),(16,5,24),(16,33,32),(8,33,32). SetFlag(4,0), flags40, AL=true. |
| N10 view short-circuit / visibility independent of result | N9 but frustum replies false,false,AL=2 | only first three points queried, SetFlag(4,1), flags44, AL=true. All-false also returns true as N9. Turning flag200 on skips all five tests and both GetPositions. |
| N11 repeat Init and Shutdown | after N1; Init again, then Shutdown twice | repeated Init: AL=true, no calls/stores. First Shutdown expands Activate(0): flag1 clear, M lookup only, stop with ID2222 retained, forced-view, delay0/clear400 =>206; then initialized0, empty list reset, store O=null before O.Release, clock=null before K1.Release, T=null, AL=true. Second Shutdown's Activate is uninitialized no-op; no further refs/sound. elapsed remains3. |
| N12 re-Init after cleanup | after N11; J clock now K2; no template/occupant | Init stores1, zero grid, K2.AddRef/store (old null); flag20 clear leaves stored pos; Activate1 notifies M, no sound (null T), forced-view, delay0/clear400 =>207. Effect and final sound skip null T. AL=true, ID2222 and elapsed3 preserved; no old K1/O/T Release. |
| N13 equal/null clock and attached-after-stop refs | first Init with old clock K1 and return K1, versus old K0 and return null; other common inputs | equal: no clock AddRef/store/Release; null: store null then K0.Release, no new AddRef, still complete Init. Separately after N11 SetOccupant(O) acquires/stores O; subsequent Shutdown sees initialized0 and leaves O retained; direct destructor Releases it. |
| N14 shutdown payload prefix after deactivation | initialized1, flags307, ID1111, controlled payloads [A,B], O and K1 retained | Activate0: stop, forced-view, delay clear, flags306. Then clear initialized; CheckFlag100, A.detach(null), A.Release; next B likewise; only then free nodes/reset count, null+Release O, null+Release K1, template=null. False detach and Release result17 do not suppress later operations; IDs/elapsed retained. Nonreturning/reentrant payload behavior is outside this trace. |
| N15 active versus tickable and sound | flags7,3,5,6,407,207 in separate valid initialized fixtures; IsActive and IsTickable, then direct PlaySFX(1) | IsActive AL values 1,0,0,0,0,1; IsTickable forwards values 1,1,1,0,1,1. Sound selects/starts in all six fixtures; it has no active/enabled/visibility/delay gate. Flag200 forces visibility testing outcome in View, not sound suppression. |
| N16 Init with active bit already set | initialized0, flags203, delay9; common other inputs | clock acquisition/replacement as N1; initial Activate1 does nothing because bit1 already set; Effect QI with empty lists only Releases R; final sound selects1111 and sends start/position once. Final flags203 (visibility still clear), delay9, initialized1, Init AL=true. IsActive false, IsTickable true. |

This extends the original thirteen rows with sixteen independent composition
rows; it does not convert proposed vectors into execution evidence. L10's old
phrase “base occupant-bounds primitive” is corrected: that helper is owned, and
its literal outputs now derive from a single occupant output call plus the
null-lot branch (`00516EFB–00516F07`, `00515D2E–00515D4E`). L1/L7 still omit
enough inputs to remain projections, even though the three selected helpers
now have bounded contracts.

The R2 self-audit preserves L2–L9 and L11–L13's bounded conclusions: source
reference stores precede old Releases (`00516898–005168A4`, `00517F5D–00517F66`),
Shutdown nulls occupant/clock before Release (`00516A72–00516A89`), and final
Release at count0/1 dispatches slot39 without a prior count store
(`00517F10–00517F27`). QI's S adjustment/output-before-AddRef and miss untouched
output remain §3, with no template ref acquisition or implicit list-payload
ownership added. Tick's ret12 and strict-exceed comparison
(`00514E13–00514E29`) differ from IsExpired's stored greater-or-equal comparison
(`00517EB0–00517EC5`). Exact rational evaluation of binary32 3A83126F times1000
gives 1073741875/1073741824 before operation rounding. At PC=24/53/64 with
nearest-even, L9's stored delta is1; upward rounding can store the next binary32
above1. This analytic qualification is not a measurement of runtime PC/RC.
L10's arithmetic was retained, while its formerly ambiguous owned/foreign
boundary was corrected above. No R1 native/game result existed to reinterpret.

## 11.2. Secondary Read: local mutation and reference chronology (R3 ADDS)

`005180E0–005181E6` is secondary slot4 of `00A95D74` (data reference
`00A95D84`), not a primary-C method. It preserves incoming ECX as **S=C+4**,
takes one serializer pointer, pops four bytes, and returns **full EAX 0 or 1**.
The constructor installs that table at C+4 (`00517D54`). Direct callers at
`00518279` and `00518476` establish derived Read delegation, not a base factory
or a complete derived read contract. The following order is owned by this body;
all referenced primitives are opaque calls with independently supplied replies.

| Order | Primitive or owned operation | Destination / consequence |
|---|---|---|
| 1 | serializer +0x1C, pointer to local word | header/version-like output is never tested or used by this body |
| 2 | test S+8 (C+0x0C); if nonnull, store null there, then old occupant.Release | no identity comparison with the incoming serialized reference; Release result ignored |
| 3 | `SC4ReadGZSerializable` `005D9730`, arguments serializer, ID `26BB767E`, address S+8, word 0 | writes directly into occupant field; no subsequent local AddRef or SetOccupant call; helper return ignored |
| 4 | serializer +0x3C, address S+0x24 | C+0x28 embedded string; return ignored |
| 5 | vector extraction `005DA550`, serializer and address S+0x10 | C+0x14 position; raw three-word output boundary; return ignored |
| 6 | serializer +0x30 twice | addresses S+0x4C then S+0x50 = C+0x50 elapsed then C+0x54 lifetime |
| 7 | serializer +0x24 three times | addresses S+0x38, S+0x3C, S+0x44 = C+0x3C sound ID, C+0x40 flags, C+0x48 count |
| 8 | unsigned index initially zero; while index < current S+0x44 | each iteration initializes a local reference-output word to null, calls `005D9730` with ID `A9B40F05`, its address and word 0, then allocates/inserts a 12-byte node with that raw output as payload |
| 9 | serializer +0x48, no arguments | only its AL is tested: zero yields EAX=1; any nonzero yields EAX=0 (`005181D5–005181E0`) |

Step8 appends before the existing sentinel: reads sentinel.previous, writes
new.next=sentinel and new.previous=old-tail, then old-tail.next=new and
sentinel.previous=new (`0051819A–005181BE`). It neither clears existing nodes
nor increments the stored count. It reloads the count for each unsigned loop
comparison (`005181C5–005181CF`), but reentrant mutations are outside this
contract. No local AddRef, payload Release, null-payload rejection or reference
helper success test occurs. The list's storage allocator is foreign; only normal
successful allocations are covered. An arbitrary allocation failure is not a
supported empty-node alternative.

Thus this Read is **not transactional**. False status from any earlier primitive
does not stop later calls; a final failure result does not undo stores, occupant
Release or appended nodes. Partial writes remain exactly the primitive's supplied
mutations. This states normal returns with partial output, not exceptions, bad
storage, unwind or corrupt-stream safety. Existing template C+0x10, clock C+0x4C,
initialized byte, reference count, grid bytes and delay elapsed C+0x58 have no
direct stores in this body; effects inside called primitives are not thereby
forbidden. It does not apply SetTemplate lifetime/flag policy or recompute bounds.

Reference acquisition by `005D9730` is **callee-supplied**, not proved here.
For a retained old O and a helper that returns the same still-valid O, the local
sequence remains null-store, Release(O), helper-write(O); it is not SetOccupant's
identity no-op. This same-pointer case needs an independently held reference
across the Release. With a retaining helper, its acquisition occurs within that
later primitive; with a reference-neutral helper no acquisition may be invented.
Null old/new and valid replacement follow the same conditional old Release and
raw output store. Lifetime safety of reference-neutral results is not inferred.

## 11.3. Base lot-facing point and radius rejection (R3 ADDS)

Primary table `00A95D90` has slot22 `00516C40` and slot23 `00515A80`;
attractor `00A95E48` and generator `00A95F00` inherit these entries. Their
virtual inputs (notably generator radius) remain dispatch-dependent. Both bodies
use C. Slot22 takes one writable vec3 pointer (ret4), with no defined status
result. Slot23 takes a point pointer and nullable binary32 distance-output
pointer (ret8), returning a boolean in **AL**, not normalized full EAX.

**Lot-facing point.** `00516C46–00516C4C` constructs a local random object
with seed FFFFFFFF but never asks it for a random number. Initially null occupant
copies stored C+0x14/+0x18/+0x1C word-for-word (`00516E86–00516E98`). Otherwise
city+0x84 supplies a lot service, then service+0x38 takes the current occupant
and returns a lot pointer. A null service or null lot forwards the output pointer
to occupant+0x20; the result is not tested (`00516E71–00516E83`). Nulling the
occupant during those callbacks is not a supported fallback case.

A lot receives +0x38 with a four-word output, then +0x5C with no args for a
full-word direction. For signed words (a,b,c,d), the body shifts a and b left
four bits; increments c and d then shifts left four. These are 32-bit operations,
then **signed FILD** and binary32 stores (`00516CC9–00516D0B`). This is not
unbounded integer multiplication. For the finite exact domain below write
X0=16a, Z0=16b, X1=16(c+1), Z1=16(d+1). Directions 0..3 select these endpoints:

| Raw direction | First endpoint (x,y,z) | Second endpoint (x,y,z) | Branch |
|---|---|---|---|
| 0 | (X0,0,Z0−1) | (X1,0,Z0−1) | `00516D22` |
| 1 | (X1+1,0,Z0) | (X1+1,0,Z1) | `00516D59` |
| 2 | (X1,0,Z1+1) | (X0,0,Z1+1) | `00516D8D` |
| 3 | (X0−1,0,Z1) | (X0−1,0,Z0) | `00516DC1` |

Direction dispatch is unsigned >3 to a separate path (`00516D12–00516D1B`),
not direction modulo four. The +1/−1 constant at `00A81228` is binary32 1;
half at `00A84D2C` is binary32 0.5. In the exact domain of signed lot words
−64..63, each selected endpoint, difference and midpoint is exactly representable:
output is first + half×(second−first) per component. Native order is x difference
on the x87 stack; y/z differences stored binary32, their half-products stored
binary32, x midpoint stored binary32, then z midpoint stored/copied and y
midpoint stored (`00516DFD–00516E66`). No normalization, terrain-height or
angle primitive is called. Output write order is z, x, y. This exact-domain rule
does not generalize reassociation/precision across all signed words. Direction
outside 0..3 uses copied `00B227CC` words; their runtime value remains Unknown.
No output is claimed for that residual path here.

**Radius rejection without distance output.** The controller calls its owned
slot20 for raw min/max vectors, then virtual slot9 for radius, storing ST0 as
binary32 r (`00515A98–00515AA2`). It compares, in order, min.x−r to point.x,
min.z−r to point.z, max.x+r to point.x, max.z+r to point.z. Subtraction/addition
has no intermediate binary32 store before each x87 comparison. Finite comparisons
reject exactly when point.x < min.x−r, point.z < min.z−r,
point.x > max.x+r or point.z > max.z+r; equality passes. This is an expanded
rectangle test, **not a Euclidean radius** or clamped distance comparison.
The lower tests use status bits C0/C3 parity (`00515AB6`, `00515ACC`);
the upper tests use C0 (`00515AE1`, `00515AF7`). Unordered comparisons take
rejection if reached with masked exceptions; this is not a payload/trap claim.

Only after all four pass does CheckFlag(0x1000) run. If its AL is nonzero,
height-service global `00B43CF4` +0x34 takes point.x,z raw binary32 words and
returns ST0 h. Reject if point.y > h+5 (`00A84D28` = binary32 5;
`00515B1F–00515B36`). Equality, less and masked unordered pass that particular
test. No controller output write occurs on rejection. If the distance pointer
is null, acceptance immediately returns true AL without querying flag0x10,
position, facing or intersection (`00515B43–00515B4C`). With a nonnull distance
pointer the acceptance decision is already true; §11.4 below specifies the
separate distance-output call chain. No ambient FP/trap guarantee is added.

### New independent literals, first proof milestone

These **PROPOSED / UNEXECUTED** R rows are input fixtures and native-derived
expectations, not portable outputs or original execution. Geometry arithmetic
uses x87 PC=53 and round-to-nearest, masked exceptions, finite exact operands;
no unspecified runtime initializer is used. References identify independently
valid objects. Primitive mutations/return channels below are independent inputs.
For Read fixtures R1–R4, start from an independently supplied live C with
position=(1,2,3), elapsed3, lifetime8, template T, clock K and empty list/count0
unless the row overrides these. These are fixture values, not constructor
defaults. Every unspecified stream output leaves its destination unchanged;
the header output is0, the occupant-reference output is null unless named, and
the final status AL is0 unless named. Each list-reference output is explicitly
listed. Reference helpers are reference-neutral in R1–R3; R4 can independently
supply either neutral or retaining helpers and must record their own acquisition
events. All returned nonnull references have independent keepalives, so none
relies on a destroyed object surviving its old Release. Intermediate primitive
status words are0 unless specified and do not gate this Read's continuation.

| ID | Raw inputs | Expected local behavior |
|---|---|---|
| R1 Read replacement / full-word status | old O1 retained; helper ID26BB767E writes O2; +0x24 replies sound=0x1234, flags=0x1002, count=0; final status EAX=0x12340000 | null occupant before Release(O1), then helper-write O2; no local AddRef(O2); EAX=1 despite nonzero full status word; template/clock unchanged locally |
| R2 Read partial outputs | old O1; helper writes null and returns 0; vector primitive changes only x to −7 and returns 0; subsequent primitives leave their output unchanged except count=0; final AL=2 | Release(O1), occupant null; position=(−7,2,3); all later calls still occur; EAX=0 with stores retained |
| R3 Read append / repeat | existing list [A], count initially1; incoming count2, helper outputs B then null; repeat Read with old occupant null, count1 and helper output D; helpers reference-neutral, independently live A/B/D | first physical list [A,B,null], stored count2, second [A,B,null,D], stored count1; one allocator/node per input entry, no local payload AddRef/Release or clearing; incoming nonzero count is not incremented |
| R4 same/null occupant | independent keepalive O; old O; helper writes O, then next Read helper writes null, then third writes null; count0 throughout | first null-store/Release(O)/helper-write O even though identical; second null-store/Release(O)/helper-write null; third no old Release; helper retention remains its supplied behavior |
| R5 asymmetric lot | lot words (−2,3,4,6), directions0,1,2,3; +0x38 AL=0 with valid outputs | X0=−32,Z0=48,X1=80,Z1=112; outputs respectively (24,0,47), (81,0,80), (24,0,113), (−33,0,80); +0x38 result ignored; no height call or random selection |
| R6 radius equality / rectangle corner | flags0, stored bounds min=(−2,7,30), max=(9,8,80) from occupant+0x28; null lot service; base template radius4; no distance output | expanded xz=[−6,13]×[26,84]; points (−6,−100,26) and (13,999,84) accepted, (13.5,0,84) rejected; accepted corner disproves a circle assumption; y unused |
| R7 height threshold / early exit | R6 fixture but flag0x1000 set; height returns11; point=(13,16,84), then (13,16.5,84), then (13.5,16,84) | first true AL at equality h+5; second false AL; third false before flag/height query; no distance-output writes |

## 11.4. Accepted-point distance output and owned intersection orchestration

This is the nonnull distance-output continuation of §11.3, **not an additional
acceptance predicate**. At `00515B4E–00515B69`, CheckFlag(0x10) chooses virtual
slot22 GetLotFacingPoint versus virtual slot11 GetPosition, each with one local
vec3 output pointer. The body copies that output's x,z into center U and the
input point's x,z into V. It calls owned helper `00514E70` with (U,V,intersection
output I), ignoring y. If its AL is nonzero it calls foreign
`LengthOfLine2D` `005D76A0` with (I,V), else with (U,V). The returned ST0 is
stored binary32 to the caller's distance-output (`00515B9A–00515BBE`). The
distance does not get compared against radius. Length internals are foreign;
no square-root implementation or success policy for it is claimed here.

`00514E70–00514F57` is attributed to the base controller in the export and is
directly called by base slot23 at `00515B9A`. It is therefore included only as
the needed owned orchestration, not as a route/pathfinding algorithm. ECX=C,
three vec2 pointers U,V,I; ret12, normalized boolean **AL** only. It calls owned
slot20 again for min/max, independently of the earlier radius check. For those
returned bounds it calls the foreign `LinesIntersect2D` `005D74D0` with U,V,
the following pair of local vec2 endpoints, and **the same** output I each time:

| Call order | Endpoint1 | Endpoint2 | Call instruction |
|---|---|---|---|
| 1 | (min.x,min.z) | (max.x,min.z) | `00514EBC` |
| 2 | (max.x,min.z) | (max.x,max.z) | `00514EF0` |
| 3 | (max.x,max.z) | (min.x,max.z) | `00514F19` |
| 4 | (min.x,max.z) | (min.x,min.z) | `00514F42` |

Each endpoint is a raw two-word copy, no arithmetic. The first nonzero AL returns
true immediately; after four zero AL replies the helper returns false. No nearest
intersection ordering, degenerate/collinear policy, output initialization or
rollback is owned here. Failed primitives may mutate I. If all fail, caller
slot23 ignores I and uses U,V for length anyway. The foreign segment algorithm
remains opaque and must be supplied as individual calls, not a completed
controller intersection or radius verdict. Mutating/aliased input pointers and
callbacks changing bounds are outside the finite composition fixtures.

## 11.5. Base state callbacks and behavior rows (R3 ADDS)

Ownership comes from primary table `00A95D90` slots28–30/32:
`0051BB60`, `0051BCA0`, `0051BCE0`, `0051BAB0`. Generator inherits these;
attractor inherits tick/behavior but overrides enter/exit with `0051BFB0` and
`0051C030`. Those override bodies are not used to infer the base rules below.
There is no base callback dispatcher, behavior cache, same-behavior comparison
or implicit enter/exit transition in these four bodies.

**Shared raw boundary, not a controller-state oracle.** Enter, tick and qualifying
behavior rows push occupant C+0x0C and raw word1 for direct manager helper
`GetControllerQInfo` `0051B9D0`. The intervening city virtual +0xBC has **no
arguments** and returns the manager receiver in EAX; the two pending stack words
are consumed by the direct helper's ret8, not by the city getter. This is visible
at `0051BAEC–0051BAF9`, `0051BB7D–0051BB8A`, `0051BCB0–0051BCBD` and the
helper's ret8 at `0051BAA4`. Queue storage/creation/ref effects remain with
[the manager contract §7](automata-controller-manager.md#7-queue-access-and-normal-mutation).
Tick and behavior then explicitly call `InitQueuePoint` `00516070` on the
returned Q with current occupant as its one argument; they do not null-test Q.
They inspect raw Q bytes/vectors **after** that primitive returns. No queue-point
calculation internals are imported. Enter only null-tests Q and does not make
this additional explicit InitQueuePoint call; the manager primitive may do so
internally on a newly created entry.

| Base callback | Raw stack channels | Local normal rule / result |
|---|---|---|
| Enter `0051BCA0–0051BCD6` | A pointer, 32-bit selector; ret8 | selector !=4: no calls; selector4: manager/Q request above, if Q nonnull call `005184E0` AddToQueue(Q,A). Ignore return. Always false **AL**. |
| Exit `0051BCE0–0051BD4D` | A pointer, 32-bit selector; ret8 | selector !=4: no calls; selector4: get manager, inline current-occupant-key lookup, on match call `005172D0` RemoveFromQueue(entry+8,A). Always false **AL**. |
| State tick `0051BB60–0051BC95` | A pointer, 32-bit selector, third **unread/untyped** word; ret12 | selector !=4: no calls, false AL. Selector4: manager/Q request then explicit InitQueuePoint(Q,occupant). If Q+0x20 byte zero, call A+0x34 with null and return true AL regardless of that call's return. Otherwise compute/forward point as below and return false AL. |

Exit does not call GetControllerQInfo or create an entry. Its inline lookup
unsigned-divides occupant pointer bits by the bucket count obtained from
(manager+0x74 minus manager+0x70) arithmetic-shifted-right two, then follows
next links comparing exact keys at node+4 (`0051BCFC–0051BD29`). It stops at
the first match. The entry+8 address guard is not a valid zero-bucket/null-manager
fallback; normal storage and a nonzero bucket count are required. This is the
same local key/bucket structure used in §9, not a host-supplied complete lookup
verdict. No controlled-list/count or reference API call is directly performed
by enter/exit/tick; queue operations may acquire/release A per the manager
contract. Repeated calls repeat their local tests and primitive requests; local
code does not suppress duplicate enters or absent exits. No callback here
sets C's template, behavior pointer, activation flags or state-selector field.

**Tick point.** With Q+0x20 nonzero, query A+0xC0, no arguments, full EAX
pointer P (`0051BBB0`). A **nonnull full pointer with AL=0** still takes the
nonnull branch. Null P copies Q+8/+0x0C/+0x10 as a vec3. Nonnull P first calls
P+0x2C with a local vec3 output, then P+0x44 with no arguments for ST0 scale s.
The comparison at `0051BBFB–0051BC0A` keeps s only for ordered s>1; otherwise
replaces it with binary32 1 (masked unordered included). For finite qualified
inputs, denote position output p and direction d=Q+0x14/+0x18/+0x1C. The x87
operation/store association at `0051BC10–0051BC43` is:

- x: multiply s×d.x, add binary32 p.x without an intervening binary32 product
  store, then store binary32 x;
- y and z: separately multiply s×d.y and s×d.z, store each product binary32,
  reload it, add its binary32 position component and store binary32 result.

Then city+0xA0 supplies a service whose +0x48 gets x,z as raw binary32 words,
returns ST0 and unconditionally overwrites local **y** as binary32. Finally
A+0x28 receives that local vec3 pointer (`0051BC4F–0051BC73`); its return is
ignored. This is not the +0x150/+0x34 height channel used by bounds or the global
height receiver in radius rejection. The initially copied `00B227CC` vector is
fully overwritten in the null-P branch, or by a full-vector P+0x2C reply in
the fixtures; partial position-output replies retain the runtime-global residual.
No position result is locally stored back into C. Tick's third word remains
untyped; animation tick's two trailing words (§7) also remain untyped.

**Behavior selection, `0051BAB0–0051BB5D`.** Inputs are one binary32 threshold
argument t and a vec3 pointer p, ret8; EAX is a borrowed row pointer or zero,
not AL status. It dereferences template C+0x10 without a null check and scans
from template+0x38 to template+0x3C in **0x7C-byte** steps. An empty range
returns EAX=0 without dereferencing p. For each row B, read binary32 r=B+4.
Compare r with t; ordered r<t or unordered skips the row (`0051BAD0–0051BADC`).
Otherwise if the full word at B+0 is anything other than 4, immediately return
B. A selector4 row requests Q and explicitly initializes it as above. If byte
Q+0x20 is zero skip; otherwise calculate dx=p.x−Q.x and dz=p.z−Q.z, then
dx²+dz² and x87 square root **without binary32 intermediate stores**
(`0051BB12–0051BB2E`). Return B if this result <=r. The second comparison's
C0/C3 test also selects on masked unordered if reached (`0051BB33–0051BB36`),
unlike the first threshold comparison; no trap/payload behavior is inferred.
On skip it reloads the current template's end pointer, advances by 0x7C and
continues. Exhaustion returns zero. All references to template/rows/Q are
borrowed; this body does no local AddRef/Release, row mutation, lifetime/flag
copy or behavior installation. Repeated same-result queries still scan and,
for each reached qualifying selector4 row, request/init Q again. Normal stable
template storage and callbacks returning normally are required.

### Second milestone: independent literals and compositions

These R8–R17 are also **PROPOSED / UNEXECUTED**. Each foreign reply is one raw
operation, never a completed controller answer. Geometry uses the finite PC53,
nearest-even, masked domain of §11.3; state arithmetic adds exact rational inputs
where shown. Queue/list payloads are valid identities and callbacks do not reenter.

| ID | Independent raw inputs | Native-derived expectation |
|---|---|---|
| R8 optional distance, first intersection | R6 bounds/radius; point V=(13,999,80); flag0x10 clear; occupant+0x20 supplies U=(5,6,74); Lines reply1 AL0 writes I=(−9,−8), reply2 AL2 writes I=(9,77); Length reply ST0=5 | accepted ALtrue; both owned slot20 calls use the R6 occupant bounds; edge1 (−2,30)→(9,30), then edge2 (9,30)→(9,80); owned short-circuit after2, length receives I,V, distance5. Raw replies are independent inputs; squared separation from I to V is 4²+3²=25. |
| R9 all intersection calls fail | occupant bounds R6; point V=(4,0,43); flag0x20 clear, so occupant+0x20 supplies U=(−1,6,31); all four Lines AL0 mutate I; Length(U,V) supplies13 | all four edges in §11.4 order with min.xz=(−2,30), max.xz=(9,80); failed I ignored by slot23; squared U,V separation is 5²+12²=169, distance13 and ALtrue even though 13>radius4; no acceptance recheck |
| R10 facing-distance composition | R5 lot direction3; radius100; slot20 obtains same lot bounds; point V=(−33,0,80); flag0x10 set; four Lines AL0; Length reply0 | slot22 produces U=(−33,0,80), not occupant position; length(U,V), distance0; both bounds calls still use owned slot20/lot conversion; no whole facing/radius oracle |
| R11 enter/exit selectors and repeat | A valid; enter(A,3), enter(A,4) twice with Q nonnull; exit(A,4) with matching occupant key, then exit(A,4) with key absent; exit(A,3) | no primitive for selector3; each enter4 requests Q with O,1 then AddToQueue(Q,A); matched exit directly looks up then RemoveFromQueue(Q,A), absent exit no queue call; all six returns false AL; duplicate/ref consequences belong to individual manager primitive, not a local suppression |
| R12 tick invalid queue point | tick(A,4,0xDEADBEEF); Q Init reply writes Q+0x20=0; A+0x34(null) reply EAX=0 | invokes detach/null exactly once, no A+0xC0, scale, service or position-forward calls; returns true AL; repeated call repeats request/init/detach if byte remains0 |
| R13 tick null versus low-byte-zero pointer | Q Init writes valid byte1, point=(−7,3,11), direction=(2,0,−3); A+0xC0 returns0, then independently valid P=0x00123400; P+0x2C writes (10,20,30); P+0x44 gives0.5; city+0xA0 service+0x48 returns9 | null P forwards (−7,9,11); full nonnull P with AL0 takes position/scale branch, cap1 gives (12,9,27); each height call receives x,z before A+0x28; both false AL |
| R14 tick product-spill discriminator | valid Q, nonnull P writes (−1,−1,−1); s=1+2^−23, d.x=d.y=d.z=1−2^−24; service height11 | exact product=1+2^−24−2^−47; x add before binary32 spill yields 2^−24−2^−47, bits337FFFFE; z's product rounds to1 then add gives +0; forwarded (bits337FFFFE,11,+0). y arithmetic is overwritten by height. A uniform rounded-product implementation differs. |
| R15 behavior ordering/equality | rows B0 selector2,r2; B1 selector4,r5; B2 selector7,r9; t3; p=(−2,999,11); Q Init byte1, point=(−5,−8,7) | B0 skipped; B1 distance sqrt(3²+4²)=5 equals threshold, returns full pointer B1; t5 also B1; t=5.5 skips B1 and returns B2 without Q request; p.y unused |
| R16 behavior absent/invalid/repeat | empty valid template range with p null; then rows B1 selector4,r5 and B2 selector7,r9, t3; Q Init writes invalid byte0; repeat query; then nonempty query t10 | empty returns0 without Q/p access; two repeats each request/init Q then select B2; t10 skips both, returns0; no enter/exit callback or behavior installation is implied |
| R17 Read→facing→query→state composition | Read R1 with helper-write O2 and vector(−7,3,11), flags0x10, count0; keep preexisting borrowed template containing R15 rows, radius100; O2 lot words R5, direction3; then facing, behavior t3/p=(−2,999,11) with valid Q=(−5,−8,7), enter(A,4), tick R13 null-P using Q point, exit(A,4) with matching key | Read releases O1 before writing O2; does not replace template/clock; facing=(−33,0,80); query returns B1 with no ref acquire/installation; enter independently requests Q/AddToQueue; tick explicitly reinitializes Q and forwards (−5,9,7), false AL; exit locally finds O2 key/RemoveFromQueue and false AL; no controller-level cached behavior transition or whole-state host callback |

These seventeen new rows make **46 proposed/unexecuted fixtures** with the earlier
L1–L13/N1–N16. The earlier 29 remain UNRUN; L1/L7 remain projections. No portable
result or native execution verdict has been generated for any of them.

## 12. Body and signature ledger; unsupported domains

**Supported** means the stated local body behavior and channels are instruction
supported in the bounded domain, not a whole-owner/native-ABI completion claim.
**Partial** means only specified projections or receiver/slot provenance are
settled. **Unknown** is intentionally unavailable for implementation by guessing.
**Foreign** is a separate owner, with only raw single-call boundaries here.
Stack sizes below are callee cleanup; AL booleans do not establish full EAX.

| Base slot / body | Inputs / result channel | Status and remaining limit |
|---|---|---|
| constructor `00517D40` | ECX C, no stack args; EAX C | Supported normal allocations; allocation failures/runtime position initializer Unknown |
| destructor `00517F80` / list helper `005650A0` | C / list storage; void | Supported direct destruction versus payload ownership distinction |
| 0 QI, secondary thunk | u32 IID, writable pointer-output; AL bool; ret8 | Supported §3 |
| 1 AddRef, secondary thunk | none; u32 EAX | Supported modulo increment |
| 2 Release, secondary thunk | none; u32 EAX | Supported local branching; dynamic slot39 effects route-dependent |
| 3 Init | none; AL true | Supported stated normal inherited composition §§6,11.1; overrides, exceptional/numeric residuals Partial |
| 4 Shutdown | none; AL true | Supported stated normal inherited activation/list/reference composition; overrides/reentry Partial |
| 5 type | base purecall | Unknown base signature beyond concrete route provenance; no base implementation |
| 6 occupant getter | none; EAX borrowed pointer | Supported |
| 7 template-name getter | none; EAX borrowed name-object address | Supported |
| 8 template getter | none; EAX borrowed pointer | Supported |
| 9 radius | none; ST0 scalar | Supported base binary32 load/zero |
| 10 IsActive | none; AL bool | Supported under specified virtual flag replies |
| 11 GetPosition | output vector pointer; ret4 | Supported mutations; EAX not a boolean contract |
| 12 CheckFlag | u32 mask; EAX 0/1; ret4 | Supported |
| 13 SetFlag | u32 mask, raw byte in four-byte slot; void; ret8 | Supported exact-one predicate |
| 14 SetOccupant / 15 SetTemplate | pointer; void; ret4 | Supported §4 |
| 16 groups | base purecall | Unknown signature / derived-only operation |
| 17 SetPosition | pointer to three words; void; ret4 | Supported local copy only |
| 18 Activate | raw byte in four-byte slot; void; ret4 | Supported normal inherited sound/view composition §§6,11.1; overrides and unsupported domains remain separate |
| 19 SetLifetime | binary32 word; void; ret4 | Supported bit copy |
| 20 occupant bounds / 21 world bounds | two writable vec3 pointers; void; ret8 | Supported finite domain §8; foreign output/height semantics remain raw |
| 22 lot facing / 23 within radius | vec3 output, ret4 / point and nullable distance output, AL bool, ret8 | Supported §11.3 finite facing and rejection/null-output paths; distance composition §11.4; copied-global direction residual Unknown |
| 24 CheckTimeOfDay `006F7DC0` | two words popped; no defined result | Partial: empty ret8; typed parameter meanings Unknown |
| 25 CheckEffectTriggers | no args; no defined result; ret0 | Supported §6.2 normal list/QI/activation/source-Release CFG; foreign QI retention unspecified |
| 27 HandleViewChanged | no args; AL true; ret0 | Supported §6.3 normal outputs/predicate/store CFG and §8 finite bounds; foreign view algorithm separate |
| 26 NotifySourceDataChanged | no args; void | Supported §6 |
| 28 state tick / 29 enter / 30 exit | A, u32 selector, tick third untyped word; AL bool; ret12 / ret8 / ret8 | Supported local branches, raw arguments and finite tick association §11.5; foreign queue/position/service algorithms and tick third-word type remain Partial |
| 31 HandleRemoveAutomaton | A pointer; AL true; ret4 | Supported direct list effects and inline table lookup; foreign queue effects not closed |
| 32 behavior template | binary32 t, vec3 pointer; EAX borrowed row/null; ret8 | Supported local row scan/threshold/selector4 distance branches §11.5; foreign queue-point calculation remains separate |
| 33 IsFull | none; EAX 0/1 | Supported unsigned capacity predicate |
| 34 grid storage address | none; EAX C+0x24 | Supported; external writes/membership Foreign |
| 35 IsExpired | none; EAX 0/1 | Supported finite comparison |
| 36 shared `00648F00` | no stack cleanup; AL=true | Partial: body is constant true, semantic slot name and typed argument use Unknown; not operator-new behavior |
| 37 IsTickable | none; forwarded CheckFlag(1) | Supported |
| 38 DoAnimationTick | first binary32 word, two untyped words; void; ret12 | Partial signature; finite local arithmetic and stated inherited deactivation supported; overrides separate |
| 39 deleting cleanup `00519340` | none; no defined result | Supported calls Shutdown, destructor, heap free in order; not a Write method |
| secondary 3/4 Write/Read | S receiver, serializer; Read full EAX0/1, ret4 | Supported Read local order/stores/appends §11.2; foreign reference acquisition/stream algorithms and exceptional failure remain Partial |
| secondary 5 GetGZCLSID | none; EAX 4A18672B | Supported identity, not factory admission |
| PlaySFX `00515770` | raw low byte in four-byte slot; no defined result; ret4 | Supported §6.1 normal range/index and exact-integer coordinate domain; foreign RNG/sound internals separate |
| intersection `00514E70` | three vec2 pointers; AL bool; ret12 | Supported owned edge-call order/short circuit §11.4; foreign LinesIntersect2D arithmetic Unknown here |
| CreateTemplateInstance `0051BDF0` | unreconciled | Unknown; not studied, no closure from names or derived callers |
| template/occupant/automaton/clock/grid/message/queue/heap internals | single raw calls only | Foreign; do not count their code as base-controller behavior |

AT22 remains partial: template-instance and foreign intersection/queue-point
algorithms, tick's unused argument type and complete callback signature proof,
foreign Read reference acquisition/stream semantics, allocation/exceptional-failure
paths, runtime position initialization, exceptional and reentrant behavior,
broad FP/numeric domains and complete derived owner contracts remain unsupported.
Read's local status/output-prefix behavior and owned state/row/geometry branches
above are supported only in their stated domain. AT21 registration/upstream notification residuals,
AT23, AD3/AD4 and historical warning counts are unchanged. No ABI rows, native
execution route, default activation, family completion or 48-entry/6556-byte
coverage credit is proposed by this publication.
