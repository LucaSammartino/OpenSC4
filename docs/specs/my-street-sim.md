# My Street Sim native owner contract

## 1. Scope, evidence, and accounting

This is the separate contract for `cSC4MyStreetSim`, class `0x0BF606A5`,
advisor 30. [mysims.md §1.1](mysims.md#11-classes) explicitly excludes this
owner. The ordinary MySim record, commute algorithms, pool, pathfinder, sixth
agent slot, and historical broad shadow comparisons do not prove this contract.
The inherited MySim and Advisor methods below are typed boundaries, not newly
recovered Street algorithms. A future portable owner must make the Street
decisions described here itself; a host callback returning a whole Street next
state is not this contract.

Evidence is the canonical Windows GOG 1.1.641 export (`functions.tsv`,
`instructions.asm`, queried with `re_query.py`) and executable vtable words.
Addresses below identify instructions, not portable bindings. Decompilation is
secondary and has misleading overlapping fields; all offsets here are taken
from instructions. No native execution or portable implementation was used.

Prior dedicated-owner audit (2026-10-23): the current spec tree and lane reports,
the history of this path, and all-ref commit subjects matching MyStreet were
searched. No earlier dedicated Street owner contract was found. The matches
were consumer/shadow attribution in `mysims1`, `cores26`, and commits `a9490b4c`
and `b2412f01`; they do not establish an original owner family. Historical
`mysims1` results remain historical. This is the first dedicated contract found
by that bounded audit, not a reset of any implementation attempt counter.
Metadata's 33 entries / 5373 bytes is an attribution target to audit, not
completed owner coverage or a count of independent algorithms.

## 2. Identity and dispatch

Let **B** denote a valid complete object. Offsets are evidence notation only;
portable owners use typed state and distinct interface identities. Installed
tables at construction (`0043D605–0043D62A`) are:

| Identity | Adjustment from B | Table | Extent used here |
|---|---:|---|---:|
| Complete object / class hooks | 0 | `00A85ED0` | 17 slots |
| Advisor | +8 | `00A85E78` | 21 slots |
| Serializable | +12 | `00A85E5C` | 6 slots |
| Message target | +16 | `00A85E4C` | 4 slots |
| MySim, IID `0x0A1DA42F` | +184 (`0xB8`) | `00A85D38` | 69 slots |
| Street interface, IID `0x0BF61276` | +696 (`0x2B8`) | `00A85D20` | 6 slots |

QI `0043D0F0` clears the caller's output first. IID `0x0BF6069F`
returns B; IID `0x0BF61276` returns B+696. Each successful own branch
writes the pointer, invokes complete-object AddRef, then returns **AL=1**, pop
8. Other IDs tail-delegate to MySim QI (`00424B00`): `0x0A1DA42F`
returns B+184 and `0x8BC01FFE` returns B, with the same store/AddRef/AL order;
other IDs continue to Advisor QI. No new unsupported-ID policy is inferred
beyond that boundary. Null output and invalid/null self are unsupported; the
apparent null-self branch still dereferences self and is not a safe failure.

The five interface QI/AddRef/Release triples `0043D240–0043D320`
subtract respectively 8, 12, 16, 184, 696 and tail-call the complete-object
entry. The root AddRef/Release (`0043D150`, `0043D160`) are redirects through
shared refcount primitives, ultimately `00968B60` and `0090D997`.
They do not maintain independent counts per interface. The shared AddRef
increments the 32-bit count at B+4 and returns it in EAX (`00968B60`). Release
(`0090D997`) compares that count **unsigned** with 1: greater values decrement,
store and return it; values 0 or 1 invoke root slot 3 with flag 1 and return
EAX=0. The destructive branch does not first decrement/store the count.
Neither operation pops explicit arguments. Initial count and thread safety
are not established by Street construction; there is no independent Street count.

### 2.1 Factory and lifecycle

`0043DFA0` requests 712 (`0x2C8`) bytes, calls the constructor when allocation
is nonnull, and returns B+696 in EAX. A null allocation returns EAX=0.
There is no AddRef in this factory body. Concrete registration is proved in
the COM director constructor: `00460DA9` supplies this factory, `00460DAE`
calls `0043D220` for class `0x0BF606A5`, and `00460DB6` calls AddCls
(`0090E133`). This is distinct from the factory's returned Street interface
and from subsequent QI to MySim.

Construction (`0043D5F0`) first invokes inherited MySim construction. It
temporarily installs `00A85D08` on the Street subobject, then the six final
tables above. Its own explicit data stores are:

| Field (relative to B) | Owned value / role |
|---|---|
| `+0x2C0` | null initially, then owned reference to a fresh 20-byte helper if allocation succeeds |
| `+0x12C` | literal `0x0BFA331E`; semantic meaning not established here |
| `+0x150` | raw 32-bit `0xFFFFFFFF` |

The helper has a ref interface at `00A80810`; construction zeros its +4/+8/+12
before allocating an 8-byte storage area, sets +4 and +8 to that area, +12 to
area+8, the first byte of the area to zero, and +16 to zero. The misleading
export name of the storage allocator does not make this a message target.
If helper allocation returns null, `+0x2C0` stays null. Failure of its nested
storage allocation is unchecked and unsupported. Installing a distinct new
helper performs new.AddRef, stores it, then releases an old nonnull helper;
the old field was explicitly zeroed, so ordinary construction has no old
release. Constructor returns B in EAX. Other defaults belong to inherited
construction and are not asserted to be zero or ordinary-MySim defaults.
No copy constructor or copy contract has been established.

Advisor `Init(arg32)` (`0043D170`) receives B+8 and pop 4. It calls inherited
MySim Init with the same interface self and argument. If **AL=0**, it returns
AL=0 without the Street poststep. If AL is nonzero, it tests the 32-bit field
at `(B+8)+0x15C` = B+`0x164`. If zero, it invokes inherited
`CreateLuaTable(B)` (`00429040`); that result is not tested. It then returns
AL=1. A nonzero field skips that call. There is no Street rollback of inherited
failure effects. Upper EAX bits are not a boolean result.

Advisor Shutdown `0043D1A0` is solely a tail jump to inherited MySim Shutdown
(`00428C20`), preserving its return/cleanup contract. There is no distinct
Street detach or helper release in this wrapper.

Root slot 3 (`0043D590`) is the deleting destructor: it reinstalls the six
Street tables, releases the nonnull helper at B+`0x2C0`, calls inherited
destruction (`00429D90`) on B, then frees B iff bit 0 of its 32-bit argument is
set. It returns the old B value in EAX, pop 4; that value is not a live object
after freeing. It does not clear the helper field before Release. Exceptions,
reentrancy, destruction twice, and nonreturning callbacks are not normalized
into success/failure behavior.

### 2.2 Serialization boundary

Serializable slot 5 (`0043D220`) returns **EAX=`0x0BF606A5`**, pop 0.
Slots 3/4 (`0043D1B0`, `0043D1D0`) receive one stream pointer, pop 4:
call inherited Write/Read respectively, ignore that return, call stream
vtable +`0x48` with no explicit argument, then return **full EAX 1 if its AL
is zero, otherwise EAX 0** (`NEG AL; SBB EAX,EAX; INC EAX`).
This proves a Street wrapper, not a separate Street save-record schema, and
does not establish that inherited data contains a complete Street snapshot.

## 3. Owned behavior

### 3.1 State vocabulary and inherited surface

These raw fields are sufficient to express the Street decisions. Initial
values are **not** implied where §2 does not explicitly give them.

| Field relative to B | Street use |
|---|---|
| `+0xC0` | 32-bit state, set to 0 by Detach, 6 for pedestrian, 4/5 for recognized vehicle categories |
| `+0xF8` | retained automaton handle A; null controls scheduling |
| `+0x150` | tracking token T, explicitly initialized to −1; Detach restores −1 |
| `+0x164` | table/context token passed to script +0xAC; Init checks zero |
| `+0x1F0` | script interface L; Attach and trip update test this |
| `+0x210` | byte active flag, 0 on Detach, 1 at end of nontrivial Attach |
| `+0x240` | signed 32-bit neighborhood extent N |
| `+0x280` | timer used by Street ticker path |
| `+0x2BC` | category C, initially untouched by own constructor, 9 before Attach classification |
| `+0x2C0` | retained empty-string helper / default landmark-name object |

The complete-object slot 16 at `0043D230` returns raw EAX=[B+`0x150`], pop
0, without AddRef or modification. The folded export name
`icf::cSC4City::GetNetworkLotManager` describes another consumer, not a Street
network-lot-manager algorithm.

The MySim table differs from the ordinary MySim table **only at slots 0–2,
34, 35** (native words `00A85D38` versus `00A85080`). Slots 34/35 are the
ShowLocation/HideLocation overrides in §3.3. All other slots, including
active/slot/residence/job/location/name/status/portrait, sound, dispatch and
script-command operations, retain exactly the inherited body addresses in
[mysims.md §7.2](mysims.md#72-cisc4mysim-sub-object-vtable-0x00a85080-69-slots).
This proves neither fixed/null Street stubs nor a new ordinary actor factory.
Inherited setters/getters and their external effects remain inherited
contracts; they are not counted here as new Street state algorithms. Advisor
slots 5–20 similarly retain inherited Advisor/MySim entries, including personal
name and CommandWouldTrigger. No special Street portrait/name policy was found
in those tables.

### 3.2 Scheduling hooks

Each hook takes B, no explicit argument, pop 0, and has **no own defined return
value**. Complete-object slots 6/7/8 are replaced by these bodies:

| Entry | If A is null | If A is nonnull |
|---|---|---|
| `0043D330`, day | no calls | MySim interface +0x110 ActivateLuaContext; inherited simulate_balloons(B, 0) |
| `0043D360`, day events | no calls | same ActivateLuaContext; tail-delegate inherited simulate_day_events(B) |
| `0043D390`, month | no calls | tail-delegate inherited simulate_month(B) |

ActivateLuaContext's AL is ignored: false does not suppress the second call.
The gate is the handle, not active byte or state. Complete-object slot 9
remains inherited UpdateVariables, not the Street private trip updater.

### 3.3 ShowLocation and HideLocation

Both take the MySim identity B+184, no explicit arguments, pop 0, and are void
at the Street boundary. Their field `self+0x40` is A; `self+0x200` is the Street
interface B+696. A null A causes no calls or stores.

ShowLocation (`0043D4F0`): retain a local A via AddRef; invoke Street slot 3
AttachAutomaton(A); release the saved local A. The last action is a tail call
to A.Release; its EAX must not be reinterpreted as a Street boolean. The local
retain protects the handle across the detach performed inside Attach.

HideLocation (`0043D450`): retain a local A; invoke Street slot 4 DetachAutomaton;
ask the input-control manager `00B43DC4` +0x60 for its current control P. If P
is nonnull, AddRef P, then query P+0x14. If the full EAX equals
`0xCC23D564`, invoke manager +0x68 with argument 0. After these callbacks,
restore the saved A into the A field **only if it differs from the current
field**: AddRef saved A; store saved A; Release old field if nonnull. Thus
HideLocation normally detaches then restores A, leaving state 0, active byte
0 and tracking −1, while A remains nonnull. Finally Release P if nonnull,
then Release the saved local A. There is no early return on manager raw
failure; no boolean manager result is checked. The restoration ordering is
at `0043D4B5–0043D4E2`. Reentrant changes by callbacks are not given a general
mutation policy; the explicit reread/comparison is preserved.

### 3.4 DetachAutomaton

Street slot 4 (`0043D3A0`), self B+696, no explicit args, pop 0,
returns **AL=1 on every normal path**:

1. If A is nonnull, initialize local Q=0 and call A.QI(`0x8B99EE7C`, &Q).
   Ignore the QI AL. Invoke A+0xA0 with the message identity B+16. Reread A;
   if nonnull, clear the A field **before** releasing that handle. Store
   state=0; Release Q if nonnull. No methods of Q other than Release are used.
2. If T != −1, obtain view V from global `00B43D04` +0x58, call
   UntrackOccupant(V, T, 0) (`0046EFB0`), then store T=−1.
3. Store state=0 again and active byte=0. Return AL=1.

There is no category reset, script update, timer restart or event push here.
Null A and T=−1 are a safe no-callback path. A failed QI with Q=0 is also safe;
a false QI that supplies nonnull Q still causes Q.Release. Missing view/global
on a path that uses it is unchecked, not a defined false result.

### 3.5 AttachAutomaton

Street slot 3 (`0043E0E0`) takes one raw automaton handle X, pop 4,
returns **AL=1 on every normal path**. It first dispatches Street slot 4
DetachAutomaton on self and ignores its return. Only then does it test X and
L: if either is null, return true with Detach's effects already committed.
There is no automatic retain of the incoming argument across that first
callback. Passing the sole owned A without a separate caller-held lifetime is
unsupported; ShowLocation supplies that retain explicitly.

For nonnull X and L:

1. Compare X to the post-detach A field. If distinct: X.AddRef; store X into A;
   release the old A if nonnull. Equal handles skip this replacement sequence.
2. Local Q=0; A.QI(`0x8B99EE7C`, &Q). If its **AL is nonzero**, call
   Q+0x268(255), then Q+0x78(`0x02000000`, 1). A true result with null Q is
   unchecked/unsupported. AL=0 skips these two calls even when Q is nonnull.
   Call A+0x9C(B+16), regardless of QI result.
3. Call L+0xAC with the token at B+`0x164`. Perform the following numeric
   assignments in order. Each assignment means **three individual calls**:
   L+0x84(key), L+0x7C(binary64 value), L+0xB8(−3). All raw returns are ignored.

| Order | Key | Value |
|---:|---|---:|
| 1 | wealth | 2 |
| 2 | trip_in_transit | 1 |
| 3 | trip_at_location | 0 |
| 4 | trip_at_transit_switch | 0 |
| 5 | trip_idle | 0 |
| 6 | trip_approaching_location | 0 |

4. Obtain descriptor D = X+0x14(). Then assign, in order, value 0 to
   `trip_on_bus`, `trip_on_foot`, **`trip_on_subway`**, `trip_in_car`,
   `trip_on_train`, `trip_on_elevated_train`, `trip_on_monorail`. The native key
   says subway; an earlier decompiler label saying subway/subway purpose must
   not be silently changed to a different key.
5. Store C=9. Query D+0x58(property ID) in the ordered short-circuit table
   below, testing **AL only**. The first match wins; do not query later rows.

| Property ID(s), in exact order | Key subsequently assigned 1 | C | State |
|---|---|---:|---|
| `0x4001` | trip_on_foot | 0 | 6 |
| `0x420B` | trip_in_car | 1 | R |
| `0x4200` | trip_on_bus | 2 | R |
| `0x4201`, then `0x4215` | trip_on_train | 3 | R |
| `0x422C`, then `0x422D` | trip_on_elevated_train | 7 | R |
| `0x422E`, then `0x422F` | trip_on_monorail | 8 | R |
| none | no extra assignment | 9 | left at post-detach value |

For pedestrian, assign trip_on_foot=1, then store state=6, C=0; call A+0xEC()
and assign trip_idle=1 **only if full EAX=7**. For each matched vehicle row,
assign its key=1; get the random object via `00B43D34` +0x24, make exactly one
RandomUint32Uniform call (`00913696`); set R=5 iff the **unsigned** raw result
is `< 0x7FFFFFFF`, otherwise R=4. The car/bus branches store state then C;
the train/elevated/monorail branches store C then state
(`0043E518–0043E691`). There is no default random draw and no subway branch.

6. Assign `state` to the current signed 32-bit state converted to binary64;
   call L+0x2C(1). Invoke the Street trip updater on B (§3.7).
7. Obtain V via `00B43D04` +0x58. Prepare arguments (3, raw word at B+`0x12C`,
   0), call A+0x14() with no explicit arguments, then call
   TrackOccupant(V, returned handle, 3, word, 0) (`0046F240`). Store its full
   EAX as T, without testing failure. Store active byte=1. Release Q if
   nonnull, irrespective of its earlier QI AL. Return AL=1.

The argument order in step 7 is proved by `0043E700–0043E722`; the values
are prepared before A+0x14 but belong to TrackOccupant. They must not be
mistaken for arguments to descriptor retrieval. No residence assignment,
ordinary move-in, direct advisor event or generic commute transition is added.

### 3.6 Message chronology and acceptance

`0043DFD0` receives message identity B+16 and one message M, pop 4. First
call M+0x10() for the full 32-bit ID. Every normal path calls inherited
MySim DoMessage(B+16, M) exactly once, **after** the optional Street work.
It forwards the inherited result channel unchanged; it does not synthesize
true for recognized IDs or false for unknown IDs.

| Message ID | Street work before inherited forwarding |
|---|---|
| `0x09D92FC3` | Get raw EAX d=M+0x38(). Exactly d=2,3,5 invoke Street slot 4 DetachAutomaton; other values do nothing. Comparisons are unsigned. |
| `0x4BC42C48` | Get raw handle M+0x80(). If exactly B+696, invoke trip updater(B), then inherited simulate_balloons(B, 1). If B+184, B, null or another object's Street handle, no Street work. No timer restart on this path. |
| `0x86AD10EE` | If A nonnull and global `00B43D5C` nonnull: call that interface +0x48(). If AL nonzero, GetElapsedTime(timer B+0x280). Only **unsigned EAX >2** invokes trip updater(B), then simulate_balloons(B,1), then timer Restart. Otherwise no Street work. |
| any other ID | none |

The thresholds, identity compare and timer order are at `0043E01B–0043E053`,
`0043E067–0043E095`, `0043E0A9–0043E0D2`. A raw false external result
preserves the already executed prefix; it is not an exception rollback.

### 3.7 Street trip/environment update

`0043D6C0` is a private, void, no-argument hook taking B, pop 0. It is called
directly by Attach and the message paths above. It is **not** the inherited
MySim trip-state update algorithm in mysims §4.6.

If L is null, return with no calls. Otherwise call L+0xAC(token at B+0x164).
Construct an inherited cLocation snapshot, retrieve O=A+0x14(), and invoke
its individual SetByOccupant(O) primitive (`00431320`, `00431430`): it supplies
retained occupant, two float coordinates x/z, and two signed int32 city-cell
coordinates cx/cz. Its internals and ordinary commute decisions remain outside
this owner. A is dereferenced without a null guard when L is nonnull.

Compute h=truncTowardZero(signed N/2) using the explicit integer correction
and arithmetic shift at `0043D72C–0043D756`. Form signed raw rectangle
(cx−h, cz−h, cx+h, cz+h), with 32-bit arithmetic, no clipping. The native
function then requires three globals nonnull: `00B43DA8`, `00B43D9C`,
`00B43D70`. If any is null, release the location's retained occupant if any
and return; **the earlier L+0xAC and location work still happened**, but no
numeric assignments or L+0x2C(1) occur. Other globals below are unchecked.

#### 3.7.1 Grid outputs, in order

Define `sample(G)` as G+0x4C(x:float32, z:float32, &gx:int32, &gz:int32)
then G+0x58(gx,gz). Quantization's return is ignored; both outputs must be
validly supplied by the foreign primitive. Define `number(key, v)` as the
three calls L+0x84(key), L+0x7C(binary64(signed int32 v)), L+0xB8(−3).
The local helper `0043D1F0` uses a signed dword FILD, then a double stack
store; no intermediate float32 conversion occurs in this assignment helper.

| Order | Output key | Source and conversion | Citation |
|---:|---|---|---|
| 1 | trip_traffic_congestion | G=`00B43DA8` +0x28(); sample; zero-extend AL | `0043D77C–0043D7C0` |
| 2 | trip_time_ratio | `00B43DA8` +0x60(cx,cz,C), x87 result; native integer conversion, low EAX used | `0043D7C5–0043D7F1` |
| 3 | trip_wait_length | literal 0 | `0043D7F6–0043D804` |
| 4 | trip_pollution_air | G=`00B43D9C` +0x14(0); sample; sign-extend AX | `0043D809–0043D851` |
| 5 | trip_pollution_water | same with argument 1 | `0043D856–0043D89F` |
| 6 | trip_pollution_garbage | same with argument 2 | `0043D8A4–0043D8ED` |
| 7 | trip_crime | G=`00B43D70` +0x14(); sample; zero-extend AL | `0043D8F2–0043D93B` |
| 8 | trip_mayor_rating | G=`00B43D60` +0x30(); sample; sign-extend AL | `0043D940–0043D989` |
| 9 | trip_eq | `00B43DA4` +0x54(&G,0,0), initially G=0; sample; x87→integer, low EAX | `0043D98E–0043D9E8` |
| 10 | trip_hq | same global +0x58(&G,0,0), initially G=0; sample; x87→integer, low EAX | `0043D9ED–0043DA43` |

EQ/HQ output G and all returned grid pointers are dereferenced unchecked.
Do not invent a null-grid fallback or reuse another grid after a raw false
result. Conversions call native `_ftol2` `009EEF04`, which operates on a
qword integer and returns EDX:EAX; only its low EAX is consumed here.
Representable finite inputs support truncation toward zero; NaNs, overflow,
ambient x87 modes/exceptions and extended precision edge cases remain partial
(question ST3), not a portable-language cast default.

#### 3.7.2 Nearby occupants, percentages, landmark

Call global `00B43DB0` QI(`0x681905BD`, &R), with R initialized null; ignore
the QI AL. Construct an empty temporary retained-occupant list, then call
R+0x1C(&list, xmin, zmin, xmax, zmax, 0). R is unchecked; false with valid
nonnull output proceeds. A false/null result is unsupported.

Consume the supplied list **from the back**, using the sentinel's previous
node (`0043DAD0`). For each occupant P:

1. Call P+0xC0() for raw building handle J. If null, do not count it or query
   its other properties. Otherwise increment total t, call P+0x8C() for signed
   d. If d>=4 increment distress3; else if d>=3 distress2; else if d>=2
   distress1. Values below 2 increment none.
2. Call P+0xA4() for signed w. If w>=3 increment high; else if w>=2 medium;
   else if w>=1 low. Values below 1 increment none.
3. Query descriptor J+0x0C(), then descriptor+0x58(`0x150A`). If AL nonzero
   and no landmark has been selected, retain J as selected landmark. Once one
   is selected, later buildings still get the descriptor/property queries,
   but do not replace it. Thus the **last supplied list element that matches**
   wins, not the first supplied element. Release each list node's retained P
   after unlinking it; free that node, then reread sentinel for next iteration.

Counters are dwords; no saturation or overflow guard is present. After the
list, form `q = float32(1.0 / (signed(t) + binary64(0.0001)))`.
For each counter k, calculate x87 `(signed(k) * q) * float32(100.0)` in that
order, invoke native integer conversion and consume low EAX. The **explicit
float32 spill is q**, not each multiplication. The binary64 constant is
bits `0x3F1A36E2EB1C432D` (`00A84E60`), multiplier 100 is float32
`0x42C80000` (`00A80AA0`); instructions verified with bytes at
`0043DBA8–0043DBCA`. Do not replace with integer k*100/t or remove epsilon.
Write outputs in order: `trip_distress1_p`, `trip_distress2_p`,
`trip_distress3_p`, `trip_wealth_low_p`, `trip_wealth_med_p`,
`trip_wealth_high_p`, then `trip_landmark_nearby` = selected?1:0
(`0043DBCF–0043DCA7`). Numeric edge domains remain ST3.

If a landmark was selected: allocate a 20-byte string helper initialized
from J+0x30() and assign it as `trip_landmark_name` using the inherited
single-value helper `00425B80`. That helper selects the key, AddRefs the
supplied string object, obtains a type token via `00B43CEC` +0x144 followed by
returned-interface +0x58(1), supplies (object,type) to L+0x114, then calls
L+0xB8(−3). These raw returns are unchecked; failed allocation yields an
unchecked null dereference in that helper, not an empty-name fallback.
Read the selected landmark's descriptor J+0x0C(), query descriptor
+0x18(`0xAA59670C`, &local property storage), then pass that returned property
to `00B43D64` +0xCC with &funding(float32), initially funding=0. Multiply the
resulting float32 funding by float32(100), native-convert and write
`trip_landmark_funding_p`. The type/semantic meaning of that property is not
inferred from its ID. The local temporary storage's internal representation
is not an owned Street algorithm.

If no landmark: assign `trip_landmark_name` from the retained default helper
at B+0x2C0 using the same helper; write funding=0. This is an object assignment,
not a fabricated null string or a direct C-string push. The own branch and
funding prefix are at `0043DCAC–0043DD5C`.

#### 3.7.3 Disasters and finalization

Obtain signed count n from global `00B43D3C` +0x40(). Initialize three byte
flags fire=riot=any=0. If n>0, visit indices 0..n−1 using manager
+0x44(index, &D, &x, &z); initialize D=null before each call and ignore its raw
return. For each returned signed x/z, test membership in the same rectangle,
**inclusive on all four edges**. Inside sets any=1; if riot still false,
call D+0x14() and compare full EAX to `0xCA380B06`, assigning riot accordingly.
Then if fire still false, call D+0x14() again and compare full EAX to
`0x621CDA33`, assigning fire accordingly. When both were false the ID may
therefore be queried twice for the same object; no cached-ID substitution.
Outside the rectangle there is no ID query. After processing the entry,
Release D if nonnull. Stop early once all three flags are true; otherwise
increment index and compare against the original n. No repeated count query.
A missing D inside the rectangle on a needed query is unchecked; outside,
null D is safe. This is proved by `0043DD61–0043DE4C`.

Write binary64 0/1 values in order: `trip_fire_nearby`, `trip_riot_nearby`,
`trip_any_disaster_nearby`, using the same select/push/set sequence. Call
L+0x2C(1). Then Release selected landmark if nonnull; clear temporary list,
free sentinel if nonnull; Release R if nonnull; Release HQ grid if nonnull;
Release EQ grid if nonnull; Release the location's retained occupant if
nonnull (`0043DF12–0043DF92`). Raw failures do not trigger Street rollback.
The final cleanup occurs after script finalization, not before.

### 3.8 CreateMyStreetSimTool

Street slot 5 `0043D520`, no explicit args, pop 0, AL boolean: allocate 48
bytes, construct the input control (`0043E760`), AddRef if nonnull, then call
the individual input-control Init(control, B) (`0043E790`). Test AL only.
If false, Release control if nonnull and return AL=0. If true, call input
manager `00B43DC4` +0x64(control, 2), ignore that result, Release control if
nonnull, return AL=1. Null allocation still reaches Init with null self:
this is not a proved safe allocation-failure vector. The input control's
internals, window handling and placement behavior are foreign to this owner.

### 3.9 Boundary and failure discipline

The tables above specify individual calls, argument widths, raw output
channels and ordering. External callbacks include ref/QI, one script operation,
one grid/descriptor/property lookup, one location snapshot, one random draw,
one view/control/timer operation or one inherited MySim/Advisor hook. None may
return a whole Street summary or replacement state machine. No private-field
cross-call or native-wrapper binding is specified. A portable owner stores its
own A/T/C/state/flags and chooses these branches itself.

Only explicit null checks and raw-return tests above define safe failures.
Exceptions, nonreturning callees, mutation of the list during callbacks,
uninitialized foreign output pointers and invalid interface lifetimes are
unknown, not converted to ordinary false results. Own writes/callbacks in the
prefix of a raw failure remain observable. No Street persistence/copy extension
or complete inherited constructor snapshot has been established.

## 4. Proposed literal traces (UNEXECUTED)

These are independent analytical expectations, not measured native results,
portable tests, coverage, or a runtime oracle. Inputs include raw results of
individual foreign calls. A valid initialized object/interface is a premise.

| Trace | Literal setup and operation | Expected ordered observations |
|---|---|---|
| L1 | Valid B; QI `0x0BF61276`; output initially `0x11223344` | output zero; output B+696; root AddRef; AL=1, pop 8 |
| L2 | Same B through MySim identity B+184; QI `0x0BF6069F` | subtract 184; output zero; output B; root AddRef; AL=1 |
| L3 | Init inherited return EAX=`0x100`; Lua field zero | inherited call only; AL=0, no CreateLuaTable |
| L4 | Init inherited AL=2; Lua field zero; CreateLuaTable returns raw failure | inherited call; CreateLuaTable(B); AL=1 |
| L5 | Init inherited AL=1; Lua field `0x12345678` | inherited call; no CreateLuaTable; AL=1 |
| L6 | Write inherited AL=0; stream +0x48 returns EAX=`0x100` | inherited Write; stream query; full EAX=1 |
| L7 | Read inherited AL=1; stream +0x48 returns EAX=`0x102` | inherited Read; stream query; full EAX=0 |
| L8 | Destructor arg=2, helper H nonnull | reinstall tables; H.Release; inherited destruction; no free; EAX=B |
| L9 | Destructor arg=3, helper null | reinstall tables; inherited destruction; free B; EAX=old B |
| L10 | Two fresh constructions B1/B2; helper allocations H1/H2 distinct | each sets its own constants and holds its own helper; releasing H1 does not release H2 |
| L11 | Copy, exact initial refcount, initial active/position state, or nested storage allocation failure | **Unsupported**; inherited defaults/copy/domain unproved; no golden snapshot |
