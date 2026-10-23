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
trace containing an unresolved owned helper is explicitly a **projection**:
its call order is specified, but its full transitive effects are not predicted.

| Trace | Literal state / inputs | Required observable effects or boundary |
|---|---|---|
| L1 construction, initialization, repeat cleanup | construct C; assign O1 and T1; first Init returns city clock K; choose flags without 0x20; owned callbacks return normally; list initially empty; Shutdown twice | construction count 0, flags 2; assign AddRef(O1), no template ref; Init sets initialized, clears grid, AddRef(K), stores K, calls Activate(1), Effect, PlaySFX(1); first Shutdown calls Activate(0), clears initialized, resets list/count, nulls+Releases O1 then K, clears template; second still calls Activate(0), skips cleanup. Projection excludes unresolved sound/view/effect effects. |
| L2 direct destruction versus final Release | initialized=1; clock K; occupant O1; controlled list [A,B]; then direct base destructor versus Release with count=1 and base slot39 | direct destructor: Release(K), free A/B nodes and sentinel, free string, Release(O1); no A/B payload Release. Final Release calls virtual Shutdown first and then destructor/free. With inherited base activation, Shutdown's payload Releases precede occupant then clock; destructor sees null service refs. |
| L3 two templates, identity and null | local name nonempty; elapsed=3; T1 lifetime=8, flags=0x107; T2 lifetime=13, flags=2; SetTemplate(T1), same T1, T2, null | each nonnull assignment clears nonempty name, copies lifetime/flags; same T1 still copies; null only stores null. Final lifetime=13, flags=2, elapsed=3; zero template AddRefs/Releases throughout. |
| L4 occupant replacement | O1 retained; SetOccupant(O2), same O2, null | AddRef(O2), store O2, Release(O1); identity no calls; store null, Release(O2). Repeat with AddRef returning 0 and Release returning 99: same sequence and stores. |
| L5 shutdown list chronology | initialized=1; base Activate request already false; flags=0x100; controlled list [A,B]; occupant O1, clock K; payload detach returns false, payload Release returns 17 | CheckFlag(0x100), A.detach(null), A.Release; then flag check, B.detach(null), B.Release; only afterwards free both nodes, count=0, null+Release O1, null+Release K, template=null; true AL regardless of those returned values. Clear flag case omits detach only. |
| L6 removal identity and duplicates | valid nonempty manager bucket table, queue lookup miss; list [A,B,A], count=3; remove A twice, then absent A | first Release(A) precedes unlink/free of first node, count=2 and list [B,A]; second Release(A), count=1, list [B]; third no payload Release. All three return true AL; queue lookup precedes list search. Queue hit adds RemoveFromQueue before each search even for absent A. |
| L7 raw boolean | initialized=1; flags=1; Activate byte=2; no overriding SetFlag; callbacks return | comparison 1 != 2; SetFlag(1,2) clears bit; nonzero request still invokes manager notification and can enable delay. A byte-1 request with flags bit already set is a no-op. Projection for owned helpers. |
| L8 equality / wrap discriminator | flag1 set, delay clear; stored elapsed=1, lifetime=1; Tick first arg=0, other words arbitrary | sum stays 1; IsExpired returns EAX=1; tick does not call Activate(false). At elapsed=0x4F800000 (binary32 2^32), same lifetime, delta=0: same equality result, with no unsigned wrap. Lifetime=0 makes IsExpired false and tick skips lifetime addition. |
| L9 delay crossing | flags=0x401; delay elapsed=0; template delay=0.5; lifetime elapsed=0; lifetime=2; first argument=1000 | under x87 PC=53, round-to-nearest, multiply/store delta gives binary32 1; delay clears and full delta is also added to lifetime, yielding stored 1. Delay is not subtracted. Other PC/RC settings need independent qualification. |
| L10 asymmetric bounds | base occupant-bounds primitive yields min=(-2,7,30), max=(9,8,80); radius=4; extents x=20,z=64; height replies 11 then 22 | after expansion/clamp: min.xz=(0,26), max.xz=(13,64); call height(0,26) then height(13,64); final min=(0,11,26), max=(13,22,64). Conditional on exact finite arithmetic and stated primitive outputs. |
| L11 pointer-adjusted QI | count=0; output initially DEADBEEF; call via S with ID E4FDA3D4; then unknown ID | thunk subtracts 4; hit stores S, AddRef makes count1, AL=true; miss leaves output S and count1, AL=false. Repeat direct primary IID1 gives C and another acquired ref. |
| L12 position / grid separation | stored pos=(5,6,9); occupant reports (10,11,12); flags=0; SetPosition((1,2,3)), GetPosition(out), then set mask0x20 and get again | setter writes stored (1,2,3) only; first getter forwards occupant output (10,11,12); second copies (1,2,3). Neither setter/getter computes tracts; slot34 returns exact address C+0x24. |
| L13 inherited versus override | initialized=0; call Shutdown on attractor versus generator | attractor enters base and calls Activate(false) before the byte test; generator override returns true without calling base. Both constructor paths enter the same C-receiver base initializer; their virtual Init/Activate paths are not interchangeable. |

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
| 3 Init | none; AL true | Partial: owned Effect/SFX/View composition unresolved; direct state/ref/position order supported |
| 4 Shutdown | none; AL true | Partial composition via virtual Activate; direct list/reference ledger supported |
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
| 18 Activate | raw byte in four-byte slot; void; ret4 | Partial owned sound/view effects; local state/order supported |
| 19 SetLifetime | binary32 word; void; ret4 | Supported bit copy |
| 20 occupant bounds / 21 world bounds | two writable vec3 pointers; void; ret8 | Supported finite domain §8; foreign output/height semantics remain raw |
| 22 lot facing / 23 within radius | unreconciled | Unknown numeric body/signature in this supplement |
| 24 CheckTimeOfDay `006F7DC0` | two words popped; no defined result | Partial: empty ret8; typed parameter meanings Unknown |
| 25 CheckEffectTriggers / 27 HandleViewChanged | no explicit args at studied calls | Partial: call provenance only; owned bodies still require contract |
| 26 NotifySourceDataChanged | no args; void | Supported §6 |
| 28 state tick / 29 enter / 30 exit | unreconciled callback arguments/channels | Unknown beyond dispatch table; decomp navigation is not a numeric contract |
| 31 HandleRemoveAutomaton | A pointer; AL true; ret4 | Supported direct list effects and inline table lookup; foreign queue effects not closed |
| 32 behavior template | unreconciled | Unknown |
| 33 IsFull | none; EAX 0/1 | Supported unsigned capacity predicate |
| 34 grid storage address | none; EAX C+0x24 | Supported; external writes/membership Foreign |
| 35 IsExpired | none; EAX 0/1 | Supported finite comparison |
| 36 shared `00648F00` | no stack cleanup; AL=true | Partial: body is constant true, semantic slot name and typed argument use Unknown; not operator-new behavior |
| 37 IsTickable | none; forwarded CheckFlag(1) | Supported |
| 38 DoAnimationTick | first binary32 word, two untyped words; void; ret12 | Partial signature; finite local arithmetic supported; virtual deactivation composition Partial |
| 39 deleting cleanup `00519340` | none; no defined result | Supported calls Shutdown, destructor, heap free in order; not a Write method |
| secondary 3/4 Write/Read | S receiver, serializer | Partial: pointer view and Write field roles settled; existing record scope retained, live Read ownership not closed here |
| secondary 5 GetGZCLSID | none; EAX 4A18672B | Supported identity, not factory admission |
| PlaySFX `00515770`, intersection `00514E70`, CreateTemplateInstance `0051BDF0` | unreconciled | Unknown owned helper bodies; no blanket credit from public attribution metadata |
| template/occupant/automaton/clock/grid/message/queue/heap internals | single raw calls only | Foreign; do not count their code as base-controller behavior |

AT22 remains partial: full owned sound/view/effect/state/behavior/template-instance
algorithms, complete callback signature proof, Read reference chronology,
allocation/partial-failure paths, runtime position initialization, exceptional
and reentrant behavior, broad FP/numeric domains and complete derived owner
contracts remain unsupported. AT21 registration/upstream notification residuals,
AT23, AD3/AD4 and historical warning counts are unchanged. No ABI rows, native
execution route, default activation, family completion or 48-entry/6556-byte
coverage credit is proposed by this publication.
