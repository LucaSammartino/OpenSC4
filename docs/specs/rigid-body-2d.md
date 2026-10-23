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

Instruction qualification in progress; see §9 for the exact publication boundary.

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
these stores. Until the lifecycle bodies are qualified, no AddRef, Release,
same-object exclusion, pair order, remap, time-of-impact, or termination rule
is supplied by this section.

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

## 9. Per-body publication ledger

| Entry/range start | Status | Bounded credit |
|---|---|---|
| `005BA010` IsPrimaryCollision | supported | complete finite unsigned-word predicate, §3 |
| `005BA080` GetClosestApproach | supported | finite support-point selection and store staging, §4 |
| `005BA180` GetRelativeVelocityAtPoint | supported | finite contact velocity and store staging, §5 |
| `005BA2F0` TestForCollision | partial | no completed test contract yet |
| `005BA890` ResolveCollision | partial | no completed response contract yet |
| `005BAB40` CheckAgainstInfiniteMassObjects | unknown | phase traversal not yet qualified |
| `005BAEA0` SweepObjectList | unknown | lifecycle not yet qualified |
| `005BAF00` RemoveDeadObjects | unknown | lifecycle not yet qualified |
| `005BAF30` DoCollisions | unknown | cycle not yet qualified |
| `005BB6A0` RemoveFromSectorMap | unknown | sector mutation not yet qualified |
| `005BB950` constructor | partial | directly observed initialization only, §7 |
| `005BB9E0` RemoveObject | unknown | lifecycle not yet qualified |
| `005BBA40` AddToSectorMap | unknown | sector mutation not yet qualified |
| `005BBC10` AddObject | unknown | admission not yet qualified |
| `005BBFD0` AdvanceTime | unknown | integration phase not yet qualified |
| `005BC270` AdvanceTime overload | unknown | wrapper not yet qualified |
| `005B9FC0` shared helper | foreign | no simulator-owned body credit |

Foreign geometry/body integrator, allocator/container helpers, random/math
library internals, and particle/building-collision helpers are outside this
owner. The simulator's own tests, response selection, sectors, and traversal
must not be replaced by a foreign whole-algorithm callback.
