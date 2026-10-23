# Automata controller: inherited base lifecycle

## 1. Scope

Behaviour owned by the base class `cSC4AutomataController` (Windows 1.1.641),
supplementing [automata.md](automata.md) §3.5 and
[the controller-manager contract](automata-controller-manager.md). This is a
**partial** contract: the attractor and generator subclasses, the Automaton and
the manager each own their own algorithms, and only the base behaviour that a
concrete controller inherits is specified here. Every fixture in §14 is a
proposed conformance vector; none has been run against the original.

Everything below assumes valid storage, calls that return normally, and no
reentrant mutation of controller fields or list links. No null-pointer guard,
exception recovery or allocation-failure path is implied unless stated. Where a
call's return value is "ignored", the controller performs its next step
whatever that call returns. Foreign calls (services, occupant, template, manager,
sound, random, geometry primitives) are boundaries: a replacement supplies each
one as a single call with its own outputs, never as a precomputed whole-controller
result.

Floats are binary32 when stored. Several comparisons use an unrounded
intermediate (the x87 extended value) rather than the stored float; these are
called out where they matter [compat].

## 2. Pointer views and layout

**C** is the primary controller interface pointer. The serializable interface is
**S = C + 4**: QI returns this adjusted pointer for the serializable IID, and the
secondary-table QI/AddRef/Release thunks subtract 4. Saved-record field offsets
quoted against S (automata.md §3.5) are the C offsets below minus 4; they are two
views of the same fields, not overlapping storage.

| C offset | Field | Constructor value |
|---|---|---|
| +0 / +4 | primary vtable `00A95D90` / serializable vtable `00A95D74` | |
| +8 | initialized byte | 0 |
| +0x0C | occupant pointer (retained) | null |
| +0x10 | template pointer (borrowed, never AddRef'd) | null |
| +0x14, +0x18, +0x1C | stored position x, y, z (float) | three copies of the float at global `00B227CC` (runtime value unknown) |
| +0x20 | reference count, u32 | 0 |
| +0x24…+0x27 | four grid-bound bytes | 0 |
| +0x28 | embedded name string object | empty |
| +0x3C | selected sound ID, u32 (an ID, not a playback handle) | 0 |
| +0x40 | flags, u32 | 2 |
| +0x44 | controlled-automata list sentinel (12-byte nodes: next, prev, payload at +8) | self-linked |
| +0x48 | controlled-automata count, u32 | 0 |
| +0x4C | clock service pointer (retained) | null |
| +0x50 / +0x54 / +0x58 | elapsed lifetime / lifetime limit / activation-delay elapsed (float) | +0 |

Padding +9…+0x0B is not initialized. The base constructor is only reached from
the two derived constructors (§13); there is no standalone base factory.

Flag bits used by the base: 1 activated, 2 enabled, 4 visible, 0x10 use lot-facing
point for distance, 0x20 use stored position (not the occupant's), 0x40 point
bounds, 0x100 detach payloads on Shutdown, 0x200 force visible, 0x400 activation
delay pending, 0x1000 height-limit test. Names are descriptive [choice]; the bit
values are [compat].

## 3. References and QI

**QI** recognizes IID 1 and `6A1857F8` (output C) and `E4FDA3D4` (output S). On a
hit it stores the output, then calls virtual AddRef, then returns true. A miss
returns false, leaves the output untouched and takes no reference. Null output
storage is not checked.

**AddRef** increments C+0x20 (mod 2^32) and returns the new count.
**Release**: if count > 1, decrement, store and return it. If count is 0 or 1,
call virtual slot 39 (deleting cleanup) **without** storing a decremented count,
then return 0 [compat]. No atomic operation or underflow guard. Base slot 39 calls
virtual Shutdown, then the base destructor, then frees the object; final Release
therefore differs from calling the destructor directly.

## 4. Template and occupant assignment

**SetTemplate** (slot 15) stores the raw pointer at C+0x10 first. If nonnull, it
clears the embedded name (if nonempty), copies template+0x4C to lifetime C+0x54
and replaces the whole flags word C+0x40 with template+0x58. A null template
changes nothing else. Assigning the same template repeats these effects. It
never AddRefs or Releases a template, never resets the elapsed fields and does
not activate.

**SetOccupant** (slot 14): if the new pointer equals the current one, nothing
happens. Otherwise AddRef new (if nonnull), store it, Release old (if nonnull);
return values ignored. No position copy, grid update or list change.

## 5. Shutdown and destruction

**Shutdown** (slot 4, returns true) always calls virtual Activate(false) first.
If the initialized byte is clear it then returns, leaving template, occupant,
clock and list alone. Otherwise, in order:

1. clear the initialized byte;
2. walk the list from sentinel.next; for each node with payload A:
   if CheckFlag(0x100), call A slot 13 (+0x34) with null; then A.Release
   (results ignored; the next link is read after these calls);
3. free all nodes, restore the sentinel self-links, count = 0;
4. occupant field = null, then Release the old occupant;
5. clock field = null, then Release the old clock;
6. template field = null (no Release).

A second Shutdown repeats only Activate(false), which under the base Activate is
a no-op once uninitialized. A reference attached after Shutdown (e.g. a new
SetOccupant) survives further Shutdowns until destruction or reassignment.

**Base destructor**: reinstall base vtables; Release the clock if nonnull; free
list nodes and sentinel **without** releasing or detaching payloads; destroy the
name string; Release the occupant if nonnull. It does not call Shutdown, clear
the initialized byte, release the template, or null the fields before releasing.

## 6. Init, activation and flags

**Init** (slot 3, no arguments, returns true). If initialized, return at once.
Otherwise:

1. initialized = 1; zero the four grid bytes;
2. K = city+0xC0 (clock). If K differs from the stored clock: AddRef K (if
   nonnull), store K, Release old (if nonnull). Null K is accepted and replaces
   the old reference; equal pointers make no reference calls;
3. if CheckFlag(0x20): get the lot service (city+0x84); if service and occupant
   are nonnull, lot = service+0x38(occupant).
   - Lot nonnull: lot+0x38 fills cell bounds (x0,z0,x1,z1) (signed 32-bit). Then
     x = 16·x0 + 0.5·16·(x1−x0+1), z likewise, computed in 32-bit integers then
     converted (overflow unsupported). If city+0x188(x,z) is true,
     y = terrain`00B43CF4`+0x34(x,z); else y = occupant position y via
     occupant+0x20, or 0 with no occupant. Store (x,y,z).
   - No lot: with an occupant, occupant+0x20 writes directly into the stored
     position; without one the position is unchanged.
   - Flag 0x20 clear: position unchanged.
4. virtual Activate(true), virtual CheckEffectTriggers, PlaySFX(true), in that
   order.

Initialized is set before any service call; there is no rollback. Statuses of
output-producing calls are ignored: only their output mutations matter.

**Activate** (slot 18, one byte argument b, no result). Uninitialized: no-op.
Otherwise compare CheckFlag(1) with the **raw byte** b; equal is a no-op.
Otherwise:

1. SetFlag(1, b); M = city+0xBC (manager); if b ≠ 0, M+0x40(C) (unchecked M);
2. PlaySFX(b); virtual HandleViewChanged();
3. delay elapsed C+0x58 = +0; SetFlag(0x400, false);
4. if b ≠ 0 and the template is nonnull with template+0x50 > 0,
   SetFlag(0x400, true).

There is no manager removal on deactivation.

**CheckFlag** (slot 12) returns 1 if any mask bit is set (not all bits).
**SetFlag** (slot 13) sets the masked bits only when the byte argument is exactly
1 and clears them for any other value. Hence Activate(2) differs from Activate(1)
[compat]: from flags with bit 1 set, Activate(2) clears bit 1 yet runs the
nonzero branches (manager notification, sound start, delay arming).

**IsActive** (slot 10): true iff bits 1, 2, 4 are set and 0x400 is clear (queried
in that order, short-circuiting). **IsTickable** (slot 37): CheckFlag(1).
**NotifySourceDataChanged** (slot 26): with a template, SetFlag(template+0x58,
true), SetFlag(2, true), lifetime = template+0x4C, delay elapsed = 0. It ORs the
template flags in rather than replacing the word, and does not reset elapsed
lifetime or activate. Null template: nothing.

### 6.1. PlaySFX (direct helper `00515770`, byte argument b)

Called by Init and Activate (non-virtual). D = sound service global `00B43CB8`.

- D null: return (selected ID kept).
- b = 0: if the selected ID is nonzero, D+0x2C(`6A451908`, C, 0, 0, 0) (stop).
  The ID is **not** cleared, so repeated stops repeat the command; ID 0
  suppresses the stop.
- b ≠ 0 (any nonzero byte): if template is null or its sound list
  (template+0x60 begin, +0x64 end, u32 elements) is empty, return. Otherwise
  draw i = uniform(n) with n = element count from a fresh random object seeded
  0xFFFFFFFF, and store selected ID = list[i] **before** any command. Get
  position p via virtual GetPosition. Call D+0x2C(`EA451903`, ID, C, 0, 0)
  (start), then D+0x2C(`CA45190F`, C, ix, iy, iz) (position) where each
  coordinate is truncated toward zero to int32. The position command is issued
  whatever the start returns. An ID of 0 still starts.

Command returns are ignored and never stored. PlaySFX tests no flag: active,
visible, delay and suppression state do not gate it. Coordinates outside int32
are unsupported.

### 6.2. CheckEffectTriggers (slot 25, no arguments, no result)

Null template, or null effect-source global R (`00B43DC8`): return. Q = null;
call R.QI(`FA47B3F9`, &Q); if it returns false, return (a written Q is neither
used nor released).

Template+0x44 (positive list) and +0x48 (negative list) are circular lists with
an ID at node+8. Starting p = true: for each positive ID while p, p = Q+0xB8(ID);
then for each negative ID while p, p = not Q+0xB8(ID). So the result is
"all positive IDs true and all negative IDs false", with no queries after the
first failure; duplicate IDs are queried again.

If either list is nonempty, virtual Activate(p). If both are empty there is **no**
Activate call. Finally **R.Release** (not Q.Release) [compat]. With Q null and
both lists empty this is a normal path; with Q null and a nonempty list it is an
unsupported null dereference.

### 6.3. HandleViewChanged (slot 27, no arguments, returns true)

V = view global `00B43DD8`. If V or the terrain global `00B43CF4` is null, return
with no flag change. If CheckFlag(0x200), SetFlag(4, 1) and return with no further
calls.

Otherwise: GetBoundingBox(A, B) (§8); P = GetPosition(). For corner A: if
city+0x188(A.x, A.z), A.y = terrain+0x34(A.x, A.z), else A.y = P.y. Same for B.
(This resamples heights already sampled by GetBoundingBox and never updates the
stored position.) P2 = GetPosition() again (not reused from P). Visible = the
first true of the frustum test `007F7E50` on V for the points, in order:
P2, A, (B.x, A.y, A.z), B, (A.x, B.y, B.z) — at most five calls. SetFlag(4,
visible), return true.

## 7. Animation time and expiry

**DoAnimationTick** (slot 38; callee pops 12 bytes; only the first argument, a
float t, is read; the other two are untyped). delta = round32(t × 0.001f) where
0.001f is binary32 `3A83126F`.

1. If CheckFlag(1) is false, nothing changes.
2. If CheckFlag(0x400): delay elapsed += delta (stored). With a template, if the
   unrounded sum ≥ template+0x50, SetFlag(0x400, false). With no template the
   flag stays set.
3. If CheckFlag(0x400) is now false and lifetime > 0: elapsed lifetime += delta
   (the **whole** delta, even on the tick that ended the delay); if the
   unrounded sum is **strictly greater** than lifetime, Activate(false).

**IsExpired** (slot 35): lifetime > 0 and stored elapsed ≥ lifetime. So
elapsed == lifetime is expired but not deactivated by the tick [compat].
**SetLifetime** (slot 19) copies the float bits without reset or validation.

Nothing resets elapsed lifetime except construction: not Activate, not Shutdown,
not a re-Init. There is no integer tick counter and no wrap. Negative, infinite
or NaN deltas are unsupported. Because the comparisons use the unrounded sum, a
round-then-compare implementation can differ at rounding boundaries (the
precision-control setting at runtime is unknown).

## 8. Position, bounds and grid bytes

**SetPosition** (slot 17, exported under the shared symbol SetKey) copies three
floats into C+0x14..+0x1C; no notification. **GetPosition** (slot 11): with a
nonnull occupant and flag 0x20 clear, forward to occupant+0x20(out) (and its
return value); else copy the stored position. Template presence does not matter.

**GetOccupantLot** (`00515D20`): borrowed lot = (city+0x84)+0x38(occupant) when
service and occupant are nonnull, else null. **GetOccupantBoundingBox** (slot 20,
two vec3 outputs): if CheckFlag(0x20) or CheckFlag(0x40) or there is no occupant,
min = GetPosition(), max = GetPosition() (two calls); else occupant+0x28(min,
max). Then if GetOccupantLot is nonnull, its cell bounds overwrite only x and z:
min = (16·x0, ·, 16·z0), max = (16·(x1+1), ·, 16·(z1+1)); y is kept.

**GetBoundingBox** (slot 21, two vec3 outputs):

1. GetOccupantBoundingBox(min, max); r = GetRadius();
2. min.x −= r, min.z −= r, max.x += r, max.z += r;
3. clamp each of min.x, min.z, max.x, max.z independently to [0, city+0x164]
   (x) or [0, city+0x168] (z); reversed endpoints are not swapped;
4. min.y = (city+0x150)+0x34(min.x, min.z); max.y likewise at (max.x, max.z).

**GetRadius** (slot 9): template+0x20, or +0 without a template.
**GetTemplateName** (slot 7): address of the template's name object
(template+4), else the embedded name C+0x28 (borrowed). Slots 6 / 8 return the
raw occupant / template pointer without AddRef.

**Grid bytes** (slot 34, exported as GetAreaToDrawTo): returns the address
C+0x24. Construction and first Init zero them; nothing in the base computes
them. The grid owner is
[automata-support.md §2](automata-support.md#2-the-controller-grid-csc4controllergrid-compat).

## 9. Controlled-automata removal, capacity, foreign boundaries

**HandleRemoveAutomaton** (slot 31, automaton A, returns true):

1. M = city+0xBC. Look up the occupant pointer in M's queue hash table:
   bucket = (occupant bits) mod n, n = (M+0x74 − M+0x70)/4; walk the chain
   comparing the key at node+4. On a match, RemoveFromQueue(entry+8, A)
   (`005172D0`, result ignored). A null M or n = 0 is unsupported, not "empty".
2. Find the **first** list node whose payload is A. If found: A.Release, then
   unlink, free the node, count −= 1. Only one node is removed per call.

The queue step happens even when A is not in the list. This path never calls
A+0x34(null), whatever flag 0x100 says.

**IsFull** (slot 33): false without a template or when template+0x54 is 0;
otherwise count ≥ capacity (unsigned).

Foreign boundaries used by the base (one call each):

| Receiver + offset | Inputs → outputs | Base treatment |
|---|---|---|
| any interface +4 / +8 | AddRef / Release → count | ignored |
| city +0xC0, +0x84, +0xBC, +0x150, +0xA0 | → service pointer | clock retained; others borrowed |
| city +0x188 | x, z → bool | terrain y vs fallback y |
| city +0x164 / +0x168 | → float extent | clamp limit |
| lot service +0x38 | occupant → lot or null | |
| lot +0x38 / +0x5C | → four int32 cell bounds / → direction | status ignored |
| occupant +0x20 / +0x28 | → position / → min, max | status ignored |
| height service +0x34, terrain +0x34, city+0xA0 service +0x48 | x, z → float | stored as y |
| automaton +0x34 / +0x28 / +0xC0 | null (detach) / position / → object P | ignored / ignored / pointer |
| manager +0x40 | C | activation notice |
| random object | seed 0xFFFFFFFF; uniform(n) → index | index assumed in range |
| sound D+0x2C | five words (§6.1) | ignored |
| effect source R / Q+0xB8 | QI / ID → bool | §6.2 |
| frustum `007F7E50` | V, point → bool | §6.3 |
| LinesIntersect2D `005D74D0` / LengthOfLine2D `005D76A0` | §10 | |
| GetControllerQInfo `0051B9D0`, InitQueuePoint `00516070`, AddToQueue `005184E0`, RemoveFromQueue `005172D0` | §11 | manager-owned |
| SC4ReadGZSerializable `005D9730`, vec3 read `005DA550`, serializer slots | §12 | |

## 10. Lot-facing point and within-radius test

**GetLotFacingPoint** (slot 22, one vec3 output). No occupant: copy the stored
position. Otherwise get the lot (as GetOccupantLot); if the service or lot is
null, forward to occupant+0x20(out). With a lot: read cell bounds (a,b,c,d) and
direction (lot+0x5C). Let X0=16a, Z0=16b, X1=16(c+1), Z1=16(d+1) (32-bit integer
arithmetic). The output is the midpoint of the edge segment one unit outside the
lot on the facing side:

| Direction | First endpoint | Second endpoint |
|---|---|---|
| 0 | (X0, 0, Z0−1) | (X1, 0, Z0−1) |
| 1 | (X1+1, 0, Z0) | (X1+1, 0, Z1) |
| 2 | (X1, 0, Z1+1) | (X0, 0, Z1+1) |
| 3 | (X0−1, 0, Z1) | (X0−1, 0, Z0) |

out = first + 0.5·(second − first) per component (y is 0; no terrain height).
This is exact for cell bounds in −64..63. Direction values above 3 (unsigned)
yield three copies of the `00B227CC` global, whose runtime value is unknown. A
random object is constructed but never drawn from.

**IsWithinRadius** (slot 23; point p, nullable float output dist; returns bool).
GetOccupantBoundingBox(min, max) (slot 20), r = GetRadius() (stored as float).
Reject if p.x < min.x−r, p.z < min.z−r, p.x > max.x+r or p.z > max.z+r (bounds
unrounded; equality passes; a NaN comparison rejects). This is an expanded
rectangle, **not** a circle [compat]. Then, if CheckFlag(0x1000): reject if
p.y > h+5, h = terrain`00B43CF4`+0x34(p.x, p.z) (equality and NaN pass).
Nothing is written on rejection.

If accepted and dist is null, return true. Otherwise:

1. U.xz = (CheckFlag(0x10) ? GetLotFacingPoint() : GetPosition()).xz; V.xz = p.xz;
2. hit = Intersect(U, V, I) (helper `00514E70`, below);
3. *dist = LengthOfLine2D(hit ? I : U, V); return true.

The distance is not compared with the radius.

**Intersect** (`00514E70`, three vec2 pointers, returns bool) calls
GetOccupantBoundingBox again and then LinesIntersect2D(U, V, e1, e2, I) for the
rectangle edges in this order, stopping at the first true: (min.x,min.z)→(max.x,
min.z); (max.x,min.z)→(max.x,max.z); (max.x,max.z)→(min.x,max.z);
(min.x,max.z)→(min.x,min.z). The same I is passed each time, so failed calls may
leave it modified; after four failures the caller ignores I.

## 11. State callbacks and behavior selection

Base slots 28 (state tick `0051BB60`), 29 (enter `0051BCA0`), 30 (exit
`0051BCE0`), 32 (behavior `0051BAB0`). The generator inherits all four; the
attractor overrides 29 and 30. None of them caches a behavior, compares with a
previous behavior or triggers enter/exit implicitly, and none stores into the
controller.

"Request Q" below means: M = city+0xBC (no arguments), Q = GetControllerQInfo(M,
occupant, 1) (manager-owned; may create the queue entry).

| Callback | Arguments | Behaviour |
|---|---|---|
| Enter | A, u32 selector | selector ≠ 4: nothing. Selector 4: request Q; if Q nonnull, AddToQueue(Q, A). Returns false. |
| Exit | A, u32 selector | selector ≠ 4: nothing. Selector 4: look up the occupant in the manager's queue table exactly as §9 step 1 (no create); on a match RemoveFromQueue(entry+8, A). Returns false. |
| State tick | A, u32 selector, third word (unread, type unknown) | selector ≠ 4: false. Selector 4: request Q; InitQueuePoint(Q, occupant) (Q not null-checked). If byte Q+0x20 is 0: A+0x34(null), return true. Otherwise forward a point (below), return false. |

Repeated calls repeat their requests; duplicate enters and absent exits are not
suppressed locally. Reference effects on A belong to the queue primitives.

**Tick point.** P = A+0xC0() (full pointer test, so a nonnull pointer with a zero
low byte counts as nonnull). If P is null, q = Q point (Q+8..+0x10). Otherwise
pos = P+0x2C(), s = P+0x44(), s := s if s > 1 else 1 (NaN → 1); with direction
d = Q+0x14..+0x1C:

- q.x = round32(s·d.x + pos.x), the product **not** rounded first;
- q.y = round32(round32(s·d.y) + pos.y), q.z likewise with the product rounded
  to float first [compat; R14 discriminates].

Then q.y = (city+0xA0)+0x48(q.x, q.z), and A+0x28(q) (result ignored). The local
vector starts as copies of `00B227CC`; a partial P+0x2C output leaves those.

**Behavior selection** (slot 32; float threshold t, vec3 p; returns a borrowed
row pointer or 0). Rows are 0x7C bytes from template+0x38 to template+0x3C
(template not null-checked; the end pointer is re-read each step). For each row
B with r = float at B+4: skip if r < t or NaN. If the u32 at B+0 is not 4, return
B. If it is 4: request Q and InitQueuePoint(Q, occupant); if byte Q+0x20 is 0,
skip; else return B if sqrt((p.x−Q.x)² + (p.z−Q.z)²) ≤ r (computed unrounded;
NaN selects). Exhaustion returns 0; an empty range returns 0 without reading p.
No AddRef, row mutation or behavior installation.

## 12. Serialization Read (secondary slot 4, `005180E0`)

Receiver S; one serializer argument; returns u32 0 or 1. Steps, each running
regardless of earlier failures:

| Step | Operation | Destination |
|---|---|---|
| 1 | serializer+0x1C(&local) | version word, unused |
| 2 | if occupant nonnull: occupant field = null, then Release old | |
| 3 | SC4ReadGZSerializable(serializer, `26BB767E`, &occupant field, 0) | occupant written directly; no SetOccupant, no local AddRef |
| 4 | serializer+0x3C | name C+0x28 |
| 5 | vec3 read `005DA550` | position C+0x14 |
| 6 | serializer+0x30 twice | elapsed C+0x50, lifetime C+0x54 |
| 7 | serializer+0x24 three times | sound ID C+0x3C, flags C+0x40, count C+0x48 |
| 8 | for i = 0 while i < count (re-read each step): ref = null; SC4ReadGZSerializable(serializer, `A9B40F05`, &ref, 0); append a node with payload ref at the list tail | |
| 9 | serializer+0x48() | return 1 if its byte result is 0, else 0 |

Read is **not transactional** [compat]: earlier failures do not stop later steps
and a failing result undoes nothing. The list is not cleared first and the count
is not incremented by appends, so reading into a nonempty list leaves the old
nodes plus the new ones while the count is the stream value. Null payloads are
appended. Even when the stream yields the same occupant, step 2 releases it
before step 3 writes it (unlike SetOccupant's identity no-op); whether
SC4ReadGZSerializable takes a reference is that helper's behaviour (unknown
here). Template, clock, initialized byte, reference count, grid bytes and delay
elapsed are not written; no SetTemplate policy is applied.

## 13. Concrete routes

The automata director registers only the attractor (`6A1628BA`, 0x6C bytes,
constructor `005181F0`) and generator (`4A1628C5`, 0xA0 bytes, constructor
`005182B0`); both call the base constructor `00517D40`. Base GetGZCLSID returns
`4A18672B`, a serialization identity only — there is no base factory.

| Slot | Attractor `00A95E48` | Generator `00A95F00` |
|---|---|---|
| 0–2 QI / refs | base | base |
| 3 Init | override: own setup, then base Init | override: calls base when uninitialized |
| 4 Shutdown | base | override: calls base only when initialized; returns true |
| 5 type / 16 groups | override (base is pure) | override (base is pure) |
| 9 radius / 15 template / 18 activate | base | overrides `00518310` / `00517460` / `00517240` (Activate does its own work, then base Activate) |
| 24 time of day | base `006F7DC0` (empty, pops 8 bytes) | override `00515200` |
| 29 / 30 enter / exit | overrides `0051BFB0` / `0051C030` | base |
| 35 expiry / 38 animation tick | base | overrides `00518350` / `00515E60` |
| 39 deleting cleanup | `005192C0`: Shutdown, derived destructor, free | `00519340`: Shutdown, base destructor, free |

All other slots are inherited by both. Derived behaviour is not specified here.

## 14. Conformance fixtures (proposed, unexecuted)

Distinct names (C, K, O, A, B, T, …) are distinct valid objects. Foreign calls
return normally with the outputs stated; unspecified outputs leave destinations
unchanged. Numeric rows assume round-to-nearest, masked exceptions and 53-bit
x87 precision. A **projection** row fixes call order but leaves some helper
inputs open (L1, L7).

### 14.1. Lifecycle (L)

| ID | Input | Expected |
|---|---|---|
| L1 (projection) | construct; SetOccupant(O1), SetTemplate(T1); Init with clock K, flag 0x20 clear, empty list; Shutdown twice | count 0, flags 2; AddRef(O1), no template ref; Init sets initialized, clears grid, AddRef(K), Activate(1), effects, PlaySFX(1); first Shutdown: Activate(0), clear initialized, reset list, null+Release O1 then K, template null; second: Activate(0) only. |
| L2 | initialized; clock K, occupant O1, list [A,B]; direct destructor vs Release at count 1 | destructor: Release K, free nodes, free string, Release O1, no A/B Release. Final Release: Shutdown first (A, B released; O1 then K released and nulled), then destructor sees null refs, then free. |
| L3 | name nonempty, elapsed 3; T1 (lifetime 8, flags 0x107), T2 (13, 2); SetTemplate T1, T1, T2, null | each nonnull clears name and copies lifetime/flags (same T1 repeats); null stores only. Final lifetime 13, flags 2, elapsed 3; no template refs. |
| L4 | O1 held; SetOccupant O2, O2, null | AddRef O2, store, Release O1; nothing; store null, Release O2. Same with AddRef→0, Release→99. |
| L5 | initialized, Activate already false, flags 0x100, list [A,B], O1, K; detach→false, Release→17 | A.detach(null), A.Release, B.detach(null), B.Release; then free nodes, count 0, null+Release O1, null+Release K, template null; true. Flag 0x100 clear: no detach calls. |
| L6 | queue lookup misses; list [A,B,A]; remove A, A, then absent | Release A before unlink: [B,A] count 2; [B] count 1; third no Release. All true; lookup precedes list search. On a queue hit, RemoveFromQueue precedes every search even for absent A. |
| L7 (projection) | initialized, flags 1; Activate(2) | 1 ≠ 2; SetFlag(1,2) clears bit 1; manager notified; delay can arm. Activate(1) with bit 1 set: no-op. |
| L8 | flag 1, no delay; elapsed 1, lifetime 1; tick t=0 | IsExpired true; no Activate(false). elapsed = 2^32 (`4F800000`), lifetime same: same. Lifetime 0: not expired, no accumulation. |
| L9 | flags 0x401; delay elapsed 0, template delay 0.5; elapsed 0, lifetime 2; t=1000 | delta = 1; delay clears; elapsed = 1 (delay not subtracted). Upward rounding would give the next float above 1. |
| L10 | flags 0; occupant+0x28 gives min (−2,7,30), max (9,8,80); null lot service; radius 4; extents 20, 64; heights 11, 22 | min.xz (0,26), max.xz (13,64); height(0,26), height(13,64); min (0,11,26), max (13,22,64). |
| L11 | count 0; output DEADBEEF; QI via S with `E4FDA3D4`, then unknown IID | hit: output S, count 1, true; miss: output S kept, count 1, false. IID 1 via C: output C, count 2. |
| L12 | stored (5,6,9); occupant reports (10,11,12); flags 0; SetPosition (1,2,3); GetPosition; set 0x20; GetPosition | stored (1,2,3); first get (10,11,12); second (1,2,3); slot 34 returns C+0x24. |
| L13 | uninitialized; Shutdown on attractor vs generator | attractor: base, Activate(false) before the initialized test; generator: returns true without base. |

### 14.2. Init/activation compositions (N)

Common fixture unless overridden: uninitialized; flags 0x202; position (12,5,28);
elapsed 3, lifetime 8, delay elapsed 9; sound ID 3333; empty list; occupant O and
clock K0 held. T: radius 4, lifetime 8, delay 0.5, sounds [1111, 2222], both
trigger lists empty. City clock K1, manager M. O+0x20 gives (12,5,28). Random
replies 0 then 1. D, R, V, terrain nonnull; R.QI true with Q. Command/notify/ref
returns are 0 and do not matter. IDs are hexadecimal.

| ID | Change / action | Expected |
|---|---|---|
| N1 | Init | initialized, grid 0; AddRef K1, store, Release K0; no lot calls. Activate(1): flags 0x203, M+0x40(C), select 1111, start(EA451903,1111,C,0,0), position(CA45190F,C,12,5,28); forced view sets 4; delay 0; 0x400 cleared then set → 0x607. Effects: lists empty, no Activate, R.Release. PlaySFX(1): select 2222, start, position. True; flags 0x607, ID 2222, elapsed 3. |
| N2 | positive [P1,P2], negative [N1]; P1 → false | as N1 to the first start; query P1 only; Activate(0): clear bit 1, no notification, stop(6A451908,C,0,0,0), view, delay 0, clear 0x400; R.Release; PlaySFX(1) selects 2222 and starts. Flags 0x206, ID 2222: inactive yet sound started. |
| N3 | initialized, flags 0x206; positive [P1,P2], negative [N1,N2]; replies 2,7,0,3; CheckEffectTriggers | queries P1,P2,N1,N2; p false; Activate(0) equals bit 1 → no-op; R.Release. P2 → 0: only P1,P2 queried. |
| N4 | initialized, 0x206; positive [P1], negative [N1,N2]; replies 2,0,0 | Activate(1): notify, select 1111, start/position, view, delay arm → 0x607; R.Release. Both lists empty: no Activate, flags 0x206, delay 9. |
| N5 | nonempty positive list; QI false but writes Q | no queries, Activate or Release. QI true, Q null, lists empty: R.Release only. Q null with nonempty list: unsupported. |
| N6 | initialized, flags 0x207, delay 9; Activate(2) | bit 1 cleared → 0x206; notify, start 1111/position; view, delay 0, 0x400 armed → 0x606. Repeating repeats all. Activate(1) from 0x207 and Activate(0) from 0x206: no-ops. Activate(0) from 0x207: stop (ID ≠ 0), no notify, view, delay clear → 0x206. |
| N7 | PlaySFX(1), ID 3333; D null / T null / empty list | ID 3333, no calls. PlaySFX(0): one stop, ID kept; again: same stop. PlaySFX(2): select 1111, start, position. Selected ID 0: start(…,0,C,0,0) and position; later stop omitted. |
| N8 | HandleViewChanged, flags 0x202; V or terrain null | true, flags unchanged, no calls. Both nonnull, flag 0x200: flags 0x206, no other calls. |
| N9 | initialized, flags 0x40, radius 4; extents 64, 64; bounds heights 11, 22; city+0x188 false then true; terrain 33; frustum all false | bounds min (8,11,24), max (16,22,32) via two GetPosition calls; A.y 5, B.y 33 (one terrain call); points (12,5,28), (8,5,24), (16,5,24), (16,33,32), (8,33,32); flag 4 cleared → 0x40; true. |
| N10 | N9 with frustum false, false, 2 | three points; flags 0x44; true. Flag 0x200: no tests, no GetPosition. |
| N11 | after N1: Init, Shutdown, Shutdown | Init: no calls. Shutdown: Activate(0) (stop with 2222, view, delay clear → 0x206), uninitialize, null+Release O, null+Release K1, template null. Second: no refs, no sound. Elapsed 3. |
| N12 | after N11; clock K2; no template/occupant; Init | AddRef K2 (old null); position kept; Activate(1) notifies, no sound, view, delay clear → 0x207; effects and sound skipped. ID 2222, elapsed 3. |
| N13 | old K1 returned K1; old K0 returned null | equal: no ref calls; null: store null, Release K0. After N11, SetOccupant(O) holds O; Shutdown keeps it; destructor releases it. |
| N14 | initialized, flags 0x307, ID 1111, list [A,B], O, K1 | Activate(0) → 0x306 with stop/view; uninitialize; A detach+Release, B detach+Release; free nodes; O then K1 null+Release; template null. |
| N15 | initialized, flags 7, 3, 5, 6, 0x407, 0x207; IsActive, IsTickable, PlaySFX(1) | IsActive 1,0,0,0,0,1; IsTickable 1,1,1,0,1,1; sound starts in all six. |
| N16 | flags 0x203, delay 9; Init | clock as N1; Activate(1) no-op; effects R.Release only; one sound start 1111. Flags 0x203, delay 9; IsActive false, IsTickable true. |

### 14.3. Read, geometry and state (R)

R1–R4 start from C with position (1,2,3), elapsed 3, lifetime 8, template T,
clock K, empty list. Unnamed stream outputs leave destinations unchanged; header
0; occupant output null; final byte 0. Read helpers take no references unless a
row says so; every returned object is kept alive independently.

| ID | Input | Expected |
|---|---|---|
| R1 | old O1; occupant read → O2; sound 0x1234, flags 0x1002, count 0; final status 0x12340000 | null, Release O1, write O2, no AddRef; returns 1 (only the low byte counts); template/clock unchanged. |
| R2 | old O1; occupant → null; vec3 read sets only x = −7; final byte 2 | Release O1; position (−7,2,3); all later steps run; returns 0, writes kept. |
| R3 | list [A], count 1; stream count 2, refs B, null; then Read again with count 1, ref D | [A,B,null] count 2; then [A,B,null,D] count 1. |
| R4 | O kept alive; old O; reads O, then null, then null | null/Release O/write O; null/Release O/write null; no Release. |
| R5 | cell bounds (−2,3,4,6); directions 0–3 | X0 −32, Z0 48, X1 80, Z1 112; (24,0,47), (81,0,80), (24,0,113), (−33,0,80). |
| R6 | flags 0; bounds min (−2,7,30), max (9,8,80); null lot service; radius 4; no dist | box x [−6,13], z [26,84]; (−6,−100,26) and (13,999,84) accepted (corner, so not a circle); (13.5,0,84) rejected. |
| R7 | R6 with flag 0x1000, height 11; p (13,16,84), (13,16.5,84), (13.5,16,84) | true (equality); false; false with no height call. |
| R8 | R6; p (13,999,80); flag 0x10 clear; U (5,6,74); edge 1 false (writes I (−9,−8)), edge 2 true (I (9,77)); length 5 | true; edges (−2,30)→(9,30) then (9,30)→(9,80); length(I,V); dist 5. |
| R9 | R6; p (4,0,43); U (−1,6,31); four edges false; length 13 | all four edges in §10 order; length(U,V); dist 13; true although 13 > 4. |
| R10 | R5 lot, direction 3; radius 100; p (−33,0,80); flag 0x10; edges false; length 0 | U = (−33,0,80) from the facing point; dist 0. |
| R11 | enter(A,3); enter(A,4) twice, Q nonnull; exit(A,4) key present; exit(A,4) absent; exit(A,3) | selector 3: nothing; each enter: request Q, AddToQueue(Q,A); matched exit: RemoveFromQueue; absent: none; all false. |
| R12 | tick(A,4,0xDEADBEEF); InitQueuePoint sets Q+0x20 = 0 | A+0x34(null) once; no other calls; true. |
| R13 | Q valid, point (−7,3,11), direction (2,0,−3); P = null, then P = 0x00123400 with position (10,20,30), scale 0.5; height 9 | null P: forward (−7,9,11); P: s → 1, forward (12,9,27); both false. |
| R14 | P position (−1,−1,−1); s = 1+2^−23; d = (1−2^−24) each; height 11 | exact product 1+2^−24−2^−47; x = round32(product − 1) = bits `337FFFFE`; z = round32(round32(product) − 1) = +0; forwards (`337FFFFE`, 11, +0). |
| R15 | rows B0 (2, r 2), B1 (4, r 5), B2 (7, r 9); t 3; p (−2,999,11); Q (−5,−8,7) valid | B0 skipped; B1 distance 5 = r → B1. t 5: B1. t 5.5: B2 with no Q request. |
| R16 | empty range, p null; then B1 (4, r 5), B2 (7, r 9), t 3, Q byte 0, twice; then t 10 | 0 without touching Q or p; B2 each time with Q requested; 0. |
| R17 | R1 (O2, position (−7,3,11)), flags 0x10, T with R15 rows and radius 100, O2 lot R5 direction 3; then facing point, behavior(t 3, p (−2,999,11)) with Q (−5,−8,7), enter(A,4), tick (null P), exit(A,4) | Release O1 before writing O2; template/clock kept; facing (−33,0,80); behavior B1; enter AddToQueue; tick forwards (−5,9,7), false; exit removes, false. |

## 15. Slot ledger and open items

"Supported" means the behaviour above is established for the stated domain.
Stack bytes are those the callee pops; "bool" means only the low byte is defined.

| Slot / body | Signature | Status |
|---|---|---|
| constructor `00517D40` | → C | supported; `00B227CC` value unknown |
| destructor `00517F80` | | supported |
| 0 QI | IID, out → bool; 8 | supported |
| 1 AddRef / 2 Release | → u32 | supported |
| 3 Init / 4 Shutdown | → bool true | supported (base) |
| 5 type, 16 groups | pure in base | derived only |
| 6 / 8 occupant / template | → pointer | supported |
| 7 template name | → name object address | supported |
| 9 GetRadius | → float | supported |
| 10 IsActive / 37 IsTickable | → bool / u32 | supported |
| 11 GetPosition | out; 4 | supported (EAX not a status) |
| 12 CheckFlag / 13 SetFlag | mask → u32 0/1; 4 / mask, byte; 8 | supported |
| 14 SetOccupant / 15 SetTemplate | pointer; 4 | supported |
| 17 SetPosition | pointer; 4 | supported |
| 18 Activate | byte; 4 | supported |
| 19 SetLifetime | float; 4 | supported |
| 20 / 21 bounds | min, max; 8 | supported (finite) |
| 22 lot-facing point | out; 4 | supported for directions 0–3 |
| 23 within radius | point, dist or null → bool; 8 | supported |
| 24 time of day `006F7DC0` | two words; 8; empty | argument types unknown |
| 25 CheckEffectTriggers / 26 NotifySourceDataChanged / 27 HandleViewChanged | none | supported |
| 28 state tick / 29 enter / 30 exit | A, selector[, word] → bool; 12 / 8 / 8 | supported; tick third argument type unknown |
| 31 HandleRemoveAutomaton | A → bool true; 4 | supported |
| 32 behavior | float, vec3 pointer → row or 0; 8 | supported |
| 33 IsFull / 35 IsExpired | → u32 0/1 | supported |
| 34 grid bytes | → C+0x24 | supported |
| 36 `00648F00` | → bool true; pops none | constant true; meaning and arguments unknown |
| 38 DoAnimationTick | float, two words; 12 | supported; last two argument types unknown |
| 39 deleting cleanup `00519340` | | supported |
| secondary 4 Read | serializer → u32 0/1; 4 | supported (§12) |
| secondary 5 GetGZCLSID | → `4A18672B` | supported |
| PlaySFX `00515770` / Intersect `00514E70` | byte; 4 / U, V, I → bool; 12 | supported |
| CreateTemplateInstance `0051BDF0` | | not studied |

Open (tracked as AT22 in [questions/automata.md](questions/automata.md)): the
runtime value of `00B227CC`; slot 24/36 meanings and the untyped tick arguments;
CreateTemplateInstance; whether SC4ReadGZSerializable retains the references it
returns; the secondary Write body; allocation-failure, exception, reentrancy and
non-finite numeric behaviour; the runtime x87 precision/rounding setting; and
the derived controllers' own contracts.
