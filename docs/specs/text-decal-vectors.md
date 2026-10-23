# TextDecal proposed conformance vectors

All vectors below are **UNEXECUTED**. They are independent literal proposals derived from the
native instructions cited in [the contract](text-decal-occupant.md), before any implementation
outputs. Primitive replies are supplied inputs, not measured game results. Symbols A/B name
opaque manager pointers; numeric IDs are literal 32-bit primitive replies. No whole-owner callback
may supply the geometry, routing or expected trace. Floating-point examples assume masked
exceptions, round-to-nearest/even and x87 precision sufficient for the indicated exact rational
operations (24, 53 or 64 significand bits suffice for G1/G2). `bits` means raw binary32 hex.

## 1. Geometry, order and unequal extents

G1: source Q reports `(w,h)=(6,4)`, text height `1/2`, center `(11,-7)`, orientation 1,
renderer level 2, raw terrain altitude `3/2`. Both old IDs sentinel, flag 2 set, valid A/B;
RGB `(2,3,5)`, alpha `1/4`. Add replies A=17, B=29.

- Aspect = 3/2; q = `(3/2)*(1/2)*16 = 12`; stored size bits `41400000`.
  hx = 6, hz = 4, turns = 1/2 from table entry level 2, orientation 1.
- Box min `(5,r32(3/2-m),-11)`, max `(17,r32(3/2+m),-3)`;
  y bits `3FB33333`, `3FCCCCCD`. Neither half-extent equals the turns value.
- Literal trace: Q.Dimensions -> (6,4); terrain.Altitude(11,-7) -> 3/2;
  renderer.View -> level 2; write size/box before Add;
  A.Add(Q,(11,-7),12,1/2)->17; A.Color(17,(2,3,5)); A.Alpha(17,1/4);
  B.Add(Q,(11,-7),12,1/2)->29; B.Color(29,(2,3,5)); B.Alpha(29,1/4);
  occupantManager.Move(O+12,0). Final IDs (17,29).

G2: source R reports `(5,8)`, height `3/4`, center `(-5/2,13/4)`, orientation 0,
level 3, altitude -2. Other conditions as G1, but old IDs (17,29), Add replies (41,43).

- Aspect = 5/8; q = `(5/8)*(3/4)*16 = 15/2`, size bits `40F00000`.
  Before orientation swap hx=15/4, hz=6; afterwards hx=6, hz=15/4.
  Turns = 1/4, from level 3/orientation 0, not the default orientation's turns.
- Box min `(-17/2,r32(-2-m),-1/2)`, max `(7/2,r32(-2+m),7)`;
  y bits `C0066666`, `BFF33333`. This asymmetric box differs from using unswapped extents.
- Trace begins A.Remove(17), store idA=FFFFFFFF, B.Remove(29), store idB=FFFFFFFF;
  then R.Dimensions(5,8), terrain.Altitude(-5/2,13/4), renderer level 3,
  A.Add(R,(-5/2,13/4),15/2,1/4)->41, Color, Alpha,
  B.Add(same four arguments)->43, Color, Alpha, Move(O+12,0).

G3: keep G1 except orientation 2, level 0: same hx=6/hz=4, size12, turns=1/8.
Orientation 3/level1 gives turns7/8 and still no half-extent swap. No geometric formula is
inferred for invalid orientation 4 or level 4; those table accesses are outside this contract.

G4: source absent, old IDs (17,29), cached size100, saved box from G2. UpdateDecal removes
17 then29 and stores both sentinel, then stops: size100 and saved box unchanged, no dimension,
terrain, renderer, Add or Move calls. With source Q present but flag2 clear, same outcome
except the flag query occurs. No source dimension query is allowed in either case.

## 2. Setter comparisons and mutation

| Vector | Starting state and input | Required outcome |
|---|---|---|
| H0 | height1; set +0 or -0 | no store/rebuild |
| H8 | height1; set8 (`41000000`) | no store/rebuild |
| H= | height1; set1 (`3F800000`) | store input bits and full rebuild despite equality |
| Hnear | height1; set `40FFFFFF` (largest f32 below8) | accept and rebuild |
| Hnf | height1; set +/-infinity or quiet NaN `7FC00001` | reject under masked exceptions |
| Cbelow | RGB(0,0,0), dynamic1, IDs17/29; R bits `38D1B716`, G=B=0 | no stores or manager calls |
| Cequal | same start; R bits `38D1B717` | store all RGB, A.Color(17,newRGB), B.Color(29,newRGB) |
| Cabove | same start; R bits `38D1B718` | same two calls with the new value |
| Cnan | same start; R quiet NaN `7FC00001`, G=2, B=3 | unordered first comparison triggers; all input bits stored and both Color calls |
| Cstatic | RGB0, dynamic0; input (epsilon,0,0) | store RGB, then full remove/rebuild; no preliminary in-place Color |
| Azeros | old alpha+0, input -0 (`80000000`) | equality: preserve old +0 bits; no calls |
| Anan | old alpha1, input quiet NaN `7FC00001`, dynamic1, IDs sentinel/29 | store NaN bits; only B.Alpha(29,NaN) |
| O= | orientation1, input1 | no mutation or calls |
| O0 | orientation1, input0 | store0, rebuild with swapped half-extents |

Color vectors start from zero so adjacent epsilon floats remain distinguishable; subtracting epsilon
from default 255 would not test the exact boundary. Alpha -1 or 2 also stores unbounded values when
different; this owner does not implement a clamp. The primitive manager may have its own contract.

P1: dynamic1, old IDs17/29; call occupant SetPosition with `(11,99,-7)` twice. Each call copies
x/z into O+94/+98, invokes shared box translation with the full input, invokes terrain +17C
with `(11,-7,out1,out2)` (supply arbitrary outputs 123 and456), then A.Position(17,(11,-7)),
B.Position(29,(11,-7)). No owner rebuild, size write or occupant-manager Move call. Changing
the supplied terrain outputs changes none of these owner decisions. Static0 performs the same
copy/translation/query prefix, then G1-style full rebuild using the separate +34 altitude reply.

## 3. Resource references, routing and failures

R1: dynamic0, managers A0/B0, IDs17/29, source Q. SetDynamic(1). Primitive view replies
B1 for index3, A1 for index2, source dimensions and numeric inputs as G1; Add replies111/222.
Literal order:

1. Store dynamic byte1; A0.Remove(17), sentinel A; B0.Remove(29), sentinel B.
2. GetManager(3)->B1; B1.AddRef; publish B=B1; B0.Release.
3. GetManager(2)->A1; A1.AddRef; publish A=A1; A0.Release.
4. Rebuild has no old IDs to remove; Q.Dimensions; terrain(11,-7); renderer level2;
   box/size stores; A1.Add(Q,(11,-7),12,1/2)->111; Color111; Alpha111;
   B1.Add(same)->222; Color222; Alpha222; Move(O+12,0).

R2: immediately SetDynamic(1) again: no calls. Change to0: removal111 then222; GetManager(1)
for B then GetManager(0) for A, followed by the same replacement rules and rebuild. With equal
returned pointers, skip AddRef/Release but retain removal/rebuild. Raw byte1 -> byte2 also causes
removal/routing/rebuild, still selecting indices3 then2; a bool-only host API must not fabricate a
native same-value test for this raw byte case.

R3: SetText("new") with old source Q, managers A/B, IDs17/29. String assignment first;
store source=null, Q.Release, factory.Create(CC1FD62C,"new",outSource)->null. Stop.
IDs remain17/29, no Remove/Add/Move; no Q.AddRef or restore. With successful output R,
there is no explicit owner R.AddRef: rebuild removes17 then29 and adds from R. Equal text
also runs assignment/release/factory. With old source null and factory failure there is no Release.

R4: G1 but old IDs `(80000005,FFFFFFFF)`; only A.Remove(80000005). New Add replies
`(FFFFFFFF,29)`: store A sentinel, no A.Color/Alpha; still B.Add, B.Color29, B.Alpha29,
Move(O+12,0). Reverse replies `(17,FFFFFFFF)` retains A17 with no rollback. Both fail:
both sentinel, box/size still updated, final Move still occurs.

R5: no source, IDs sentinel/sentinel, dynamic0, managers A0/B0. Changed dynamic1 and
both manager lookups return null: release B0 after publishing null, then A0 after publishing
null; UpdateDecal exits for absent source. No crash behavior is proposed for later adding
with these null managers. Nonnull ID with null corresponding manager is invalid for RemoveDecals.

R6: initialized, A nonnull/id sentinel, B nonnull/id29, source Q. Shutdown:
Unsubscribe(O+8,A6B79602), Unsubscribe(O+8,06CDB65B), Unsubscribe(O+8,26D31EC1);
no A.Remove; null A then A.Release; B.Remove29 then sentinel B; null B then B.Release;
null source then Q.Release; shared prop Shutdown(O+12); return AL1. Repeated Shutdown
under the proved base-cleared-initialized protocol returns AL1 with no owner calls.
Destruction afterwards has no A/B/source Release, but destroys text and base. Direct destruction
without Shutdown instead releases B, A, Q and does not Remove/Unsubscribe. Do not execute a
second destructor on the same storage. Null-manager shutdown branches leave their IDs unchanged.

## 4. Records, receiver aliases and return channels

S1: fresh object, text ASCII "A", center `(11,-7)`, height1/2, cached size12, RGB(2,3,5),
alpha1/4, orientation `00000102`, dynamic1, IDs17/29. The outer record starts LE u16 `03 00`,
then the established embedded-prop record. Its **extension only**, hex by field:

| Field | Bytes |
|---|---|
| text | `01 00 00 00 41` |
| first x,z | `00 00 30 41 00 00 E0 C0` |
| height | `00 00 00 3F` |
| size | `00 00 40 41` |
| second x,z | `00 00 30 41 00 00 E0 C0` |
| RGB | `00 00 00 40 00 00 40 40 00 00 A0 40` |
| alpha | `00 00 80 3E` |
| orientation byte | `02` |

Read into another valid object stores orientation **2**, not258, and leaves its prior dynamic,
IDs and source/managers unchanged. There is no setter/rebuild trace. If first pair is instead
`(+1,+2)` and second is `(11,-7)`, final center is `(11,-7)`. Reading via O+20 holder thunk
and O+16 serializable yields the same fields; treating either receiver as O must fail this vector.
Likewise occupant SetPosition on O+12 writes O+94/+98 while preserving IDs17/29 at O+84/+88.

S2: version1 record adds a discarded Uint32 after the text, and the height word
`01 00 00 00` means numeric1 -> f32 bits `3F800000`, not f32 bits `00000001`.
Version2 uses the same discarded Uint32 but Float32 height; version3 omits the discarded word.
Version4 follows the version3 outer shape (no outer reject). Version1 Uint32 `FFFFFFFF`
converts to binary32 `4F800000` (2^32) under nearest/even, bypassing height-setter limits.
For v3, height raw bits `00000001` stay those bits; Read does not validate them.

S3: stream GetError returns EAX=`12340000`: AL0 means final full EAX1. EAX=`12340001`
means final EAX0. Embedded prop returning AL0 skips the extension, but final GetError AL0
still yields EAX1; this is not a conjunction with the prop result. Init and Shutdown promise
only AL1: upper EAX bits are not a bool oracle. QI's success likewise only promises AL1.
