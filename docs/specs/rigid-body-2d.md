# Native rigid-body simulator 2D — bounded contract

## 1. Status and provenance

First publication, 2026-10-23, lane `re-rigid-body-2d4`. This is the exact
`cSC4RigidBodySimulator2D` owner, distinct from the particle emitter/life model.
Canonical scope is full effects (particles, decals, visuals), REGO0, no
dependencies. This document grants no whole-owner, replacement, native execution,
default-enable, or byte-coverage credit. Supported below means a static normal-domain
instruction contract, not a native test pass. Publication may remain partial.

Sources: Windows export `ghidra/export/windows`, queried first with `re_query.py`;
local `decomp/cSC4RigidBodySimulator2D.c` used only for navigation. Names are ported
metadata (`auto:A/B/D`), not by themselves proof of semantics. Numeric rules below
are qualified against byte-bearing Windows instructions. No live procedure,
native-memory call, game run, or output oracle was used. NoApp/live UNKNOWN remains
unchanged. The [particle](particles.md) and [effects](effects.md) outlines supply
context only; their partial physics mentions do not complete this owner.

Identity independently checked against `bin/SimCity4.exe`: file version
`1.1.641.0`, PE machine `0x14C`, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
This matches the export summary hash. A local read-only audit compared exported
instruction bytes with the PE-backed bytes across the 16 owner-named ranges in
§9: zero mismatches. This is provenance, not semantic completion or native
execution. The three §3–§5 bodies comprise 33, 81, and 46 instructions respectively.

### Outline

- §2: raw records and call boundaries.
- §3: primary sector predicate.
- §4: support-point selection (the misleadingly named closest approach).
- §5: relative contact velocity.
- §6: collision test and response.
- §7: object, sector, and advancement phase boundary.
- §8: independent proposed vectors.
- §9: per-body evidence ledger and remaining work.

## 2. Raw records and call boundaries

All addresses refer to the 32-bit Windows image at base `0x00400000`. A body
record is not the simulator receiver. The small collision helpers take explicit
stack arguments; they do not read a simulator `this`. All pointers in this
contract name valid, suitably sized records; output pointers designate writable,
nonoverlapping pairs of binary32 values. Aliased, invalid, concurrently changed,
or partially initialized records are outside the admitted domain.

The constructor initializes at least `0x2C` receiver bytes. The observed list
node/body relation is body=`node+08`; it is not a COM interface pointer. A
body/registration record extends through the sector word at `+78`; allocation
and any additional trailing fields are not inferred from the collision helpers.

| Body byte offset | Positively observed role |
|---|---|
| `+00,+04` | centre/position pair used for geometric offsets |
| `+08,+0C` | linear velocity pair read and updated by response |
| `+1C` | angular velocity scalar; multiplies the perpendicular contact offset |
| `+20,+24` | two rectangle extents used in support/separation calculations |
| `+2C` | linear response multiplier (inverse-mass role); no independent mass conversion established |
| `+68` | angular response multiplier (inverse-inertia role); no independent inertia conversion established |
| `+6C,+70` | axis pair `(c,s)`; perpendicular axis is `(-s,c)` |

The geometric interpretation assumes finite, nonnegative extents and an
orthonormal axis pair supplied as raw foreign body state. Helpers do not normalize
the pair. No general validation, rollback, or reentrancy guarantee is established.
Unless a section narrows it, floating arithmetic describes ordered finite inputs
whose intermediate operations stay finite. PC/RC and any foreign math replies are
explicit preconditions for bit-exact examples, not a global native mode claim.

## 3. Primary sector predicate

`0x005BA010–0x005BA076`: three caller-cleaned 32-bit stack words, in order
`currentKey, keyA, keyB`; Boolean result in AL, plain return, no receiver/output
buffer. EAX's upper bytes are not a Boolean result contract.

Split each key into the masked high half (`key & 0xFFFF0000`) and low half
(`key & 0xFFFF`). Select the unsigned smaller high half and independently the
unsigned smaller low half, add them, then compare the reconstructed word with
`currentKey`. Return true exactly on equality. Equal halves select the first
key's temporary, with no observable numeric difference. Evidence:
`0x005BA020–0x005BA031` masks, `0x005BA03B–0x005BA05F` unsigned selections,
`0x005BA063–0x005BA070` reconstruction/equality.

This is sector duplicate suppression, not body-pointer priority. It cannot by
itself reject the same body or duplicate registration. Those admission rules need
separate proof. No signed coordinate interpretation is implied by these masks.

## 4. GetClosestApproach: directional support point

`0x005BA080–0x005BA176`: caller-cleaned stack pointers
`outXY, referenceXY, directionXY, body`; returns `outXY` in EAX after writing
two binary32 components. No time interval or timestep is an argument. It selects
one rectangle support corner; it does not calculate closest-approach time.

Let `d` be the binary32-stored dot product of `(body.position-reference)` with
`direction`. Form `u = extentX*(c,s)` with each component stored to binary32.
Form `v = extentY*(-s,c)`; its Y component is stored to binary32, while its X
component remains in the x87 register stack. If `dot(u,direction)*d` is strictly
greater than zero, negate both components of `u`. Independently, if
`dot(v,direction)*d` is strictly greater than zero, negate both components of
`v`. Zero/equality does not negate; a zero direction therefore selects the
positive-axis corner. Evidence: `0x005BA08B–0x005BA0E5`, comparisons at
`0x005BA0FB–0x005BA118` and `0x005BA12D–0x005BA14A`.

Write `(body.x+u.x)+v.x` to out X. For Y, store `body.y+u.y` to a binary32
temporary before adding `v.y` and storing out Y. X's intermediate addition has
no matching binary32 store. The writes occur X then Y at
`0x005BA14E–0x005BA170`. Algebraic regrouping or uniform host double arithmetic
does not establish identical last bits.

## 5. Relative velocity at a supplied point

`0x005BA180–0x005BA20D`: caller-cleaned stack pointers
`outXY, A, B, pointXY`; EAX returns `outXY`. There are no body mutations.

The returned vector is **B-at-point minus A-at-point**. A body's contact
velocity is its linear velocity plus `angularVelocity * (-(point.y-body.y),
point.x-body.x)`. This sign and argument order follow
`0x005BA18B–0x005BA1F4`; differences and X-then-Y output stores are at
`0x005BA1F8–0x005BA207`.

The routine is not a uniform two-vector expression evaluated at one precision.
B's two point offsets are stored binary32 (`0x005BA19D`, `0x005BA1AD`). A's
Y rotational term is stored binary32 (`0x005BA1BD`), then its Y contact velocity
is stored binary32 (`0x005BA1CF`). A's X contact velocity remains in x87.
B's X rotational term and X contact velocity are stored binary32
(`0x005BA1E0`, `0x005BA1EE`); B's Y contact velocity is stored binary32
(`0x005BA1F4`). Subtract these staged values, then round each output on its
binary32 store. This is sufficient for the exact-integer proposed vector in §8;
arbitrary last-bit claims require explicit PC/RC and operation-stage evaluation.

## 6. Collision test and response

### 6.1 TestForCollision: finite contact-selection CFG

`0x005BA2F0–0x005BA88C`: caller-cleaned stack pointers `A, B, outNormalXY,
outPointXY`; no simulator receiver. Treat **AL** as Boolean: early separation
clears only AL at `0x005BA49B`; the final success/failure paths set EAX to 1/0
at `0x005BA878`/`0x005BA886`. Do not use all of EAX on every return path.

For notation, axes are `a=(cA,sA)`, `aPerp=(-sA,cA)` and similarly `b,bPerp`;
extents are `ax,ay,bx,by`. Store to binary32:

- `u = dot(B.position-A.position,a)` and `v = dot(B.position-A.position,aPerp)`.
- `C = cA*cB+sA*sB` and `S = cA*sB-sA*cB`.

The four separation quantities are tested in this order. Any strictly positive
quantity returns false immediately, without writing the output pairs. Equality
continues. Inputs here exclude NaNs; the unordered x87 paths are not generalized
into a normal-domain Boolean claim.

| Order | Separation quantity | Decision citation |
|---|---|---|
| 1 | `abs(u) - ((abs(C*bx)+abs((-S)*by))+ax)` | `005BA3CE–005BA3D9` |
| 2 | `abs(v) - ((abs(C*by)+abs(S*bx))+ay)` | `005BA406–005BA411` |
| 3 | `abs(C*u+S*v) - ((abs(C)*ax+abs(S)*ay)+bx)` | `005BA450–005BA45B` |
| 4 | `abs(C*v+(-S)*u) - ((abs(C)*ay+abs(S)*ax)+by)` | `005BA48E–005BA4A3` |

These expressions specify operation/sign choices; no general bit-exact output
oracle is asserted. `u,v,C,S` are binary32 intermediates at
`0x005BA340/350/387/393`; each separation expression is evaluated in x87
without a binary32 store immediately before its comparison.

If all four pass, calculate four support points with §4, in order, **before**
evaluating any candidate: `Q1=support(A.position,a,B)`,
`Q2=support(A.position,aPerp,B)`, `Q3=support(B.position,b,A)`,
`Q4=support(B.position,bPerp,A)` (`0x005BA4A4–0x005BA523`). Initialize the
binary32 best score to bits `0x7CF0BDC2` (approximately `1e37`).

For each candidate define `r = Q-referenceBody.position`, transverse projection
`h`, signed face projection `t`, and normal `n` as below. The offset components
are stored binary32 before computing `t`; `t` is also stored binary32 before
the velocity gate. Compute `U = GetRelativeVelocityAtPoint(A,B,Q)` only after
the transverse gate passes. The gate scalar is `dot(n,U)*t`.

| Candidate | reference | transverse gate | `t`, selected normal | velocity gate | Branch anchors |
|---|---|---|---|---|---|
| Q1 | A | `abs(dot(r,aPerp)) <= ay` | `dot(r,a)`, `a` | scalar `<= 0` | `005BA561–005BA5CD` |
| Q2 | A | `abs(dot(r,a)) <= ax` | `dot(r,aPerp)`, `aPerp` | scalar `<= 0` | `005BA61D–005BA68D` |
| Q3 | B | `abs(dot(r,bPerp)) <= by` | `dot(r,b)`, `b` | scalar `>= 0` | `005BA6F2–005BA75E` |
| Q4 | B | `abs(dot(r,b)) <= bx` | `dot(r,bPerp)`, `bPerp` | scalar `>= 0` | `005BA7BD–005BA82D` |

Q1, if eligible, writes outputs and installs score `abs(t)` without comparing
it to the sentinel. Q2–Q4 write only if eligible **and** `abs(t)` is strictly
less than the current score (`0x005BA695–005BA69E`,
`0x005BA766–005BA76F`, `0x005BA835–005BA83E`). Equal scores retain the earlier
candidate. The score is a distance projection, not a time or penetration depth.
Normals preserve their axis signs; they are not automatically oriented A-to-B.

Every accepted candidate writes point then normal. Q1 writes point Y then X
(`0x005BA5D7/5DD`); Q2–Q4 write point X then Y. All write normal X then Y.
After all candidates, return true exactly when the binary32 score is strictly
less than the **binary64** constant at `0x00AA0FA8`, value `1e35`
(`0x005BA867–0x005BA88C`). If none qualifies, outputs are unchanged. With
huge admitted finite projections, an accepted candidate may write outputs yet
the final threshold returns false: false does not universally mean untouched
outputs. Consumers must check AL before using the result.

### 6.2 ResolveCollision: bounded noncoincident response

`0x005BA890–0x005BAB31`: caller-cleaned pointers `A,B,normalXY,pointXY,
extraVelocityXY`. No receiver and no specified return value. Distinct body
records, independent read-only input pairs, finite intermediate values, and a
strictly positive denominator are the admitted response domain. No validation
of those preconditions is claimed.

Store the two contact offsets as `(point.x-body.x, -(point.y-body.y))` in
binary32. Let `kA=offsetA.x*n.y+offsetA.negY*n.x`, and likewise `kB`.
Call §5 with **B,A,point**, giving `VAatPoint−VBatPoint`; add the supplied
extra velocity componentwise. The call and reverse order are at
`0x005BA8A5–0x005BA8D1`. Neither restitution nor a force callback is supplied
by a host algorithm.

Compute the impulse numerator as `dot(VAatPoint−VBatPoint+extra,n)` multiplied
by the binary32 constant bits `0xBFB33333` at `0x00AA0FB0`
(`-1.399999976158142`, not an exact real -1.4). Denominator operation order:
first `(mA+mB)*(n.x*n.x+n.y*n.y)`; add `(kB*iB)*kB`; then add
`(kA*iA)*kA`. Here `m` and `i` are the raw `+2C` and `+68` multipliers.
Divide, then store the quotient `j0` to binary32 (`0x005BA94D–0x005BA94F`).

Reload `j0`. If it is nonnegative, add binary32 constant bits `0x38D1B717`
at `0x00A99E1C`; if negative, subtract that constant. Its exact real value is
`0.00009999999747378752`. This produces adjusted `j` kept in x87, with no
immediate binary32 store (`0x005BA959–0x005BA97C`). In particular a zero
quotient takes the positive adjustment; it is not an early return.

Apply mutations in this exact field order, each body-field store binary32:

1. A velocity X, then Y: add `(j*mA)*n`.
2. B velocity X, then Y: subtract `(j*mB)*n`.
3. A angular velocity: add `(kA*iA)*j`.
4. B angular velocity: subtract `(kB*iB)*j`.

Evidence: `0x005BA97C–0x005BA9F8`. Each linear Y impulse component is stored
to a binary32 temporary before addition/subtraction; the corresponding X
component stays in x87. Angular crosses are recomputed from the saved offsets.
The normal need not be unit length for these literal operations; no implicit
normalization occurs here.

Only **after** these velocity updates, calculate centre difference `D=A.pos−B.pos`
and its length using native FSQRT. D.y is stored binary32 before its square;
D.x stays in x87. Store length to binary32 `L`, but the `<4` branch compares
the still-register length against binary32 4 (`0x005BA9FA–0x005BAA23`). If
length is at least 4, return without position changes. Otherwise, for a positive
nonzero length, form `W=(4−L)/sqrt(dot(D,D))*D`, storing its components
binary32, and normalized `N=D/sqrt(dot(D,D))` (N.x in x87, N.y binary32).
The magnitude squares are recomputed with the stored D.y; normalization is
guarded by a strict-positive squared-length test at `0x005BAA80–0x005BAAC0`.

Then update A velocity X,Y by `+N`, A position X,Y by `+W`, B velocity X,Y
by `−N`, B position X,Y by `−W`, in that order
(`0x005BAADA–0x005BAB21`). This second correction is not weighted by `m` or
`i`; callers cannot assume a zero response multiplier prevents it. The
coincident-centre path constructs a random object with raw seed `0xFFFFFFFF`
and calls foreign `RandomDirection2` (`0x005BAA3C–0x005BAA60`); its random
contract and lifetime remain partial. No deterministic coincident-centre result
is published. There is no rollback promise after any preceding mutation.

## 7. Object/sector and advancement boundary

The constructor `0x005BB950–0x005BB9DF` takes the simulator receiver in ECX,
returns it in EAX, and has no explicit stack argument. It creates three separate
self-linked roots at receiver `+00,+04,+08` using requests of `0x8C` bytes,
then a self-linked root at `+0C` using a `0x28`-byte request. It clears receiver
`+14,+18,+1C,+20`, invokes a shared helper with receiver `+10` and argument
100, then writes binary32 64 at `+24` and binary32 1/64 at `+28`.
Evidence: `0x005BB956–0x005BB9D5`.

The helper at `0x005B9FC0` has an automatically ported hash-map constructor
name, but is called here with a global ECX and a size. That name does not prove
that each root is itself a hash map, or give owned-body credit for the helper.
Allocation policy, failure, destruction and retention are not inferred from
these stores. No AddRef/Release or destructor contract is inferred from storage.

### 7.1 Bounded removal lifecycle

`SweepObjectList` (`0x005BAEA0–0x005BAEF4`) receives one 32-bit stack pointer
to a root-pointer cell, ignores ECX, and returns with a four-byte pop. The root's
first word is the first node; node `+00,+04` are next and previous. Visit forward
until the root. If node byte `+7E` is nonzero, retain it and advance. If zero,
save next, write `previous.next=next`, then `next.previous=previous`, then free
the allocation whose raw address is `node−4`. Read the allocator-class byte at
`node−1`. Values below 9 select global pointer table `0x00B62674[index]` and
call virtual offset `+04` on that allocator, passing `node−4`. Otherwise call
`0x009F015B` with that pointer and caller cleanup. Resume at the saved next node.
Evidence: `0x005BAEB0–0x005BAEEE`. The ported name on `0x009F015B` does not
make this a rigid-body Release. There is no body-method call in this body.

`RemoveDeadObjects` (`0x005BAF00–0x005BAF20`) uses an ECX simulator receiver
and no stack arguments. It sweeps receiver root cells in order **+08, +04,
+00**, then returns. Empty/self-linked lists produce no free calls. Ordinary
completion of each foreign deallocator, stable roots, and absence of callback
reentrancy are preconditions; failure/destruction behavior remains unknown.

`RemoveObject` (`0x005BB9E0–0x005BBA34`) uses ECX simulator plus one stack
body pointer, pops four bytes, and has no specified return value. First traverse
the receiver `+0C` root forward. For every node whose raw pointer at `+08` **or**
`+0C` equals that body pointer, unlink it (previous.next then next.previous),
call foreign `0x0090CF63(node)` with caller cleanup, and resume from saved next.
All matching nodes are removed; nonmatching nodes retain their order
(`0x005BB9F3–0x005BBA18`). Then, if body word `+78` is nonzero, call
`RemoveFromSectorMap(simulator,body)`. Finally write body byte `+76=0`
(`0x005BBA1B–0x005BBA32`). Removal from the object list is deferred to a sweep:
the advance body independently establishes `body=node+08`, so this flag is
the sweep's node `+7E`. No object-release callback is proved by these operations.
The internals and side effects of the sector-removal call remain unqualified.

### 7.2 Exact advancement boundary, partial inner body

The public overload at `0x005BC270–0x005BC296` takes ECX simulator plus a
single **binary32 timestep word** on the stack, returns with a four-byte pop,
and specifies no return value. Its order is:

1. Call inner `AdvanceTime` with root-cell pointer `simulator+00` and the
   timestep bits unchanged.
2. Call the same body with `simulator+04` and the same bits.
3. Call `DoCollisions` on the simulator receiver.

The wrapper has no timestep sign/zero check, retry, remaining-time variable,
substep loop, or automatic dead sweep. This does not establish whole-call normal
outcomes for zero/negative/nonfinite time. The inner body and collision cycle
remain partial; a host must not supply their entire algorithms as an oracle.

The inner body (`0x005BBFD0–0x005BC26B`) takes ECX simulator plus root-cell
pointer and binary32 dt, and pops eight bytes. Its entry checks for an empty
list and otherwise visits nodes forward. A zero byte at node `+7E` skips the
body; a live body is node `+08` (`0x005BBFD3–0x005BBFF8`). At its qualified
prefix, it updates body X,Y by old velocity times dt, then body `+10` by
`dt*body[+14]`, then binary32 angle `+18` by `dt*body[+1C]`, in that order
(`0x005BBFFE–0x005BC03D`). This is owned arithmetic, not an external whole
integrator callback. Angle wrap/trigonometry, acceleration/drag and exact sector
coordinate conversion remain outside the completed contract.

At the qualified tail, compare a computed raw new key with body `+78`.
Equality skips remapping. If different and old key nonzero, call
`RemoveFromSectorMap`; then store the new key at body `+78` **before** calling
`AddToSectorMap`. Advance to next list node (`0x005BC22E–0x005BC25C`). The
coordinate calculation and insertion internals are not replaced with guessed
floor/hash rules. This prefix/tail publication does not complete the inner body.

DoCollisions is a separate no-stack-argument ECX body. Its observed entry
iterates a container at simulator `+10` and begins a sector's pointer range at
offset `+08`, ending at `+0C`; it starts an inner pointer at outer+4
(`0x005BAF3F–0x005BAF8B`). This establishes a within-sector pair-loop prefix,
not complete pair enumeration or duplicate suppression. Full cycle ordering,
the `+0C` pair-like list, `CheckAgainstInfiniteMassObjects`, and unnamed
helper `0x005BA210` remain partial/unknown. No first-contact time, simultaneous
tie, step cap, repeat guard, or whole-step trace is inferred from their names.

## 8. Independent proposed vectors

All rows are **PROPOSED/UNEXECUTED**, derived from instruction-stage arithmetic
before any future implementation outputs. They are not native conformance passes.

| ID | Raw input | Required result and distinction |
|---|---|---|
| RB-P1 | keyA=`0x00020009`, keyB=`0x00050003` | reconstructed primary key=`0x00020003`; current at either original key is false; swapping A/B does not change the selected key |
| RB-P2 | keyA=keyB=`0xFFFF0000`, current equal | true; high-half comparison is unsigned |
| RB-S1 | body centre `(10,20)`, axes `(1,0)`, extents `(2,3)`, reference `(0,0)`, direction `(1,0)` | support `(8,23)`; second projection is zero so its positive extent survives |
| RB-S2 | same body/reference, direction `(0,0)` | `(12,23)`; zero is not a sign-flip branch |
| RB-V1 | A centre `(1,2)`, velocity `(3,4)`, angular velocity 2; B centre `(-2,1)`, velocity `(-1,5)`, angular velocity -1; point `(4,6)` | A contact velocity `(-5,10)`, B `(4,-1)`, output `(9,-11)`; swapped bodies give `(-9,11)` |
| RB-V2 | same bodies/point, both angular velocities zero | output `(-4,1)`, distinguishing relative linear velocity from contact velocity |

All arithmetic in these rows is exactly representable binary32, so rounding
control does not change these finite stage results. No claim about invalid
pointers, uninitialized output buffers, or unsupported time inputs is tested.

### 8.1 Contact-selection trace (PROPOSED/UNEXECUTED)

Use A centre `(0,0)`, B centre `(3,0)`, both axes `(1,0)`, extents `(2,1)`,
and zero angular velocities. The separation values are `−1,−2,−1,−2`.
The four supports are Q1=`(1,1)`, Q2=`(5,1)`, Q3=`(2,1)`, Q4=`(2,1)`.
Let `U=B.velocity−A.velocity`.

| ID | U | Candidate trace / result |
|---|---|---|
| RB-C1 | `(0,0)` stationary | Q1 accepted, score 1; Q2 fails transverse gate; Q3 and Q4 qualify but tie score 1; true, normal `(1,0)`, point `(1,1)` |
| RB-C2 | `(-1,0)` approaching along X | same selected contact; zero transverse velocity also passes Q4's gate |
| RB-C3 | `(1,0)` separating along X | Q1 and Q3 fail velocity gates, Q2 fails transverse gate, Q4 qualifies at velocity equality; true, normal `(0,1)`, point `(2,1)` |
| RB-C4 | `(1,-1)` | Q1 and Q3 fail velocity gates; Q2 fails transverse gate; Q4 fails its velocity gate; false, outputs retain their supplied prior values |
| RB-C5 | `(0,1)` parallel to the facing X edge | Q1 accepted at equality; Q3/Q4 cannot displace its equal score; true with C1's contact |

RB-C3 is why a general “separating means false” replacement is wrong. For
RB-C6 move B to `(4,0)` and keep U zero: X separation is exactly zero and
continues. Q1 initially has score 2; Q3 ties it; Q4 has score 1 and wins,
giving true, normal `(0,1)`, point `(2,1)` (tangent boundary accepted). For
RB-C7 move B to `(5,0)`: first separation is +1, false with unchanged outputs.
These literal traces use only exact small-integer operation stages. They do not
exercise registration, sectors, or a whole collision cycle.

### 8.2 Asymmetric response-stage vector (PROPOSED/UNEXECUTED)

RB-R1 directly calls §6.2 with A centre `(0,0)`, velocity `(3,4)`, angular
velocity 2, `mA=1/2`, `iA=1/8`; B centre `(4,0)`, velocity `(-1,5)`, angular
velocity −1, `mB=1/4`, `iB=1/4`. Normal `(1,0)`, point `(1,2)`, extra
velocity `(1,0)`. This is a raw response-stage example, conditional on these
inputs; it does not claim §6.1 chooses this contact.

A's contact velocity is `(-1,6)`, B's is `(1,8)`. The reversed helper reply is
`(-2,-2)`, then extra gives `(-1,-2)`. Both cross scalars are −2. The exact
denominator is `3/4 + 1 + 1/2 = 9/4`; `j0` is binary32 `0x3F1F49F4`
(`2609789/4194304`). After the positive epsilon adjustment,
`j = 85531309847/137438953472`.

Conditional rounding precondition: x87 PC at least 53 significand bits, RC
nearest/even, ordinary finite instructions. Every arithmetic operation after
`j0` in this example is exact before the documented binary32 stores. Proposed
stored results are:

| Field | binary32 bits | Value |
|---|---|---|
| A velocity X | `4053EA10` | `3.3111610412597656` |
| B velocity X | `BF93EA10` | `-1.1555805206298828` |
| A angular velocity | `3FEC15F0` | `1.8444194793701172` |
| B angular velocity | `BF3057BF` | `-0.6888388991355896` |

Y velocities remain 4 and 5. Positions stay unchanged because the centre length
is exactly 4, taking the early return. The quotient, epsilon and these outputs
were independently derived with exact rational stages and binary32 rounding;
no implementation or native output was sampled.

RB-R2 (PROPOSED/UNEXECUTED): retain centres, normal, point and response
multipliers from RB-R1, but set both linear and angular velocities and the extra
velocity to zero. Denominator remains 9/4, quotient compares equal to zero, and
adjusted `j` is **positive** epsilon, not zero. With PC/RC as above, first-stage
linear X updates are `+epsilon/2` for A and `−epsilon/4` for B; angular updates
are `−epsilon/4` for A and `+epsilon/2` for B. All Y and position values are
unchanged. RB-R3 replaces RB-R1's extra X=1 with X=3: numerator changes sign,
binary32 `j0=−2609789/4194304`, and `j=−85531309847/137438953472`, taking
the negative epsilon branch. These are response-stage sign/equality examples.

RB-R4 (PROPOSED/UNEXECUTED): A centre `(0,0)`, B centre `(2,0)`, zero
velocities/angular velocities, normal `(1,0)`, point `(0,0)`, extra `(0,0)`,
both `m=1/2`, both `i=0`. After the small epsilon impulse, centre difference is
`(-2,0)`, length is exactly 2, and the correction has W=`(-2,0)`, N=`(-1,0)`.
A position becomes `(-2,0)` and B `(4,0)`, separation 6; the correction moves
**both** bodies by the full `(4−length)` amount. A velocity X is binary32
`epsilon/2−1`, B velocity X is binary32 `1−epsilon/2`, Y values zero.
This illustrates the unweighted second correction, rather than assuming a
mass-weighted positional solver or a final separation of exactly 4.

### 8.3 Lifecycle/phase traces (PROPOSED/UNEXECUTED)

RB-L1: three empty self-linked roots at +08,+04,+00: RemoveDeadObjects performs
three sweeps in that order and makes no deallocation calls. RB-L2: a +08 root
with nodes N1(live), N2(dead), N3(dead), each dead allocation-class byte below
9: retain N1, unlink N2, allocator callback on N2−4, unlink N3, allocator callback
on N3−4; then sweep +04 and +00. Each callback is assumed to return normally
without list reentrancy. These are allocator callbacks, not proved releases.

RB-L3: RemoveObject with +0C entries `(body,X)`, `(Y,body)`, `(Y,X)` removes
the first two in list order before sector-removal call (if body+78 is nonzero),
then clears body+76. Raw pointer identity is the criterion. RB-A1: the public
wrapper's three-call trace is inner(+00,dt), inner(+04,dt), DoCollisions; this
trace is conditional on normal completion and is not a whole-step output vector.

No proposed empty/single/multiple-sector pair trace, duplicate-registration
outcome, time-of-impact interval vector, dt-sign outcome, cycle tie, or cap vector
is published without the missing owner branches; see RB1–RB6 in the
[open questions](questions/rigid-body-2d.md).

## 9. Per-body publication ledger

| Entry/range start | Status | Bounded credit |
|---|---|---|
| `005BA010` IsPrimaryCollision | supported | complete finite unsigned-word predicate, §3 |
| `005BA080` GetClosestApproach | supported | finite support-point selection and store staging, §4 |
| `005BA180` GetRelativeVelocityAtPoint | supported | finite contact velocity and store staging, §5 |
| `005BA2F0–005BA88C` TestForCollision | supported | finite separation/contact CFG and output semantics, §6.1; general bit-exact/NaN behavior excluded |
| `005BA890–005BAB31` ResolveCollision | partial | finite noncoincident response and mutation order, §6.2; coincident random branch incomplete |
| `005BAB40` CheckAgainstInfiniteMassObjects | unknown | phase traversal not yet qualified |
| `005BAEA0–005BAEF4` SweepObjectList | supported | normal stable-list deletion and raw allocator-call order, §7.1 |
| `005BAF00–005BAF20` RemoveDeadObjects | supported | three-sweep order, §7.1 |
| `005BAF30–005BB388` DoCollisions | partial | within-sector pair-loop prefix only, §7.2 |
| `005BB6A0` RemoveFromSectorMap | unknown | sector mutation not yet qualified |
| `005BB950` constructor | partial | directly observed initialization only, §7 |
| `005BB9E0–005BBA34` RemoveObject | partial | pair unlink, sector-call boundary, deferred-dead flag, §7.1 |
| `005BBA40` AddToSectorMap | unknown | sector mutation not yet qualified |
| `005BBC10` AddObject | unknown | admission not yet qualified |
| `005BBFD0–005BC26B` AdvanceTime | partial | live-list prefix and remap-call tail, §7.2 |
| `005BC270–005BC296` AdvanceTime overload | supported | exact three-call phase boundary, §7.2 |
| `005B9FC0` shared helper | foreign | no simulator-owned body credit |

External geometry/body-operation internals, allocator/container helpers, random/math
library internals, and particle/building-collision helpers are outside this
owner. The simulator's own tests, response selection, sectors, and traversal
must not be replaced by a foreign whole-algorithm callback.
