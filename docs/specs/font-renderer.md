# Font renderer: class 08CDA30B

Instruction-derived contract for the default software font renderer in Windows GOG 1.1.641.
Written by re-font-renderer4, 2026-10-23. Sources: static export queried with `re_query.py`,
original PE vtable/constant data, and existing decompilation used for navigation. FIRST1's live REA
returned `target_unavailable` / **No app is open**; live health this round is UNKNOWN, with no retry.
No native execution or implementation comparison supports this document.

## Outline

| Section | Subject |
|---|---|
| 1 | Identity, ownership and exact body inventory |
| 2 | Typed slots and lifetime |
| 3 | Batch dispatch and record layout |
| 4 | Target access, clipping and effect order |
| 5 | Pixel modes and packing |
| 6 | Literal traces and limits |

## 1. Identity, ownership and exact body inventory

The director registers **08CDA30B with factory 00895F65** (`00895FB2–00895FBE`).
That factory allocates **8 bytes**, returns null on allocation failure, otherwise tail-calls
`00897722` (`00895F65–00895F7A`). The constructor installs **00AC55B4**, clears the dword
reference count at +4, and initializes the shared blend table once (§5). Independently, successful
`cGZFont::Init` allocates 8 bytes and calls this same constructor for its default renderer,
stores it at font +8C and calls AddRef (`0089962A–00899654`). The allocation-failure path there
stores null and then dereferences it; the factory's clean null return does not imply safe font Init failure.

The auto-labelled constructor `cGZDrawFontRenderer::cGZDrawFontRenderer` at **00829F70** is a
different, larger holder: factory `0082A2B0` allocates **450 hex bytes**, it installs **00AC14BC**,
and asks COM for **08CDA30B / A8BC2A75 into member +43C** (`00829FAE–00829FE6`). Its associated
class getter `00829910` returns **8B3255F9**. This pairing is instruction-proved: the drawing
director pushes factory `0082A2B0`, calls that getter, pushes its result, then calls AddCls
(`00825BAB–00825BB8`). Its QI `00829920` also accepts A8BC2A75 and 1;
sharing an interface does not make these the same object. Its Release `00829960` decrements to
zero then calls its own deleting-destructor slot +20, unlike the small object's direct free.
This establishes the separate pointer views, without specifying that holder's drawing/cache system.
There is also a concrete delegation edge: with its +444 flag set, the holder's unlock body
loads member +43C and calls its +18 with the original three arguments (`00829A20–00829A44`).
Its draw body loads +43C and calls +1C with seven arguments (`0082A878–0082A8B3`). These are
foreign callers of the small interface, not extra small-object slots.
The inherited **4,444-byte labelled owner is unaudited** and is not renderer coverage credit.

Raw PE dwords at `00AC55B4–00AC55D0` give the eight slots below; `00AC55D4` is zero and the
font vtable starts at `00AC55D8`. A zero alone is not a universal vtable-boundary rule: here the
constructor, QI, consumers and stack cleanups establish this eight-slot interface.

Exact body inventory for this contract (sizes are export body bytes, inclusive ranges, each counted once):

| Address | Bytes | Role / classification |
|---|---:|---|
| 00895F65 | 22 | class factory; exclusive path audited here |
| 00897722 | 38 | small constructor; exclusive |
| 0089620D | 37 | QI; exclusive |
| 008961F6 | 23 | Release; exclusive |
| 00896232 | 17 | lock callback; exclusive |
| 00896243 | 17 | unlock callback; exclusive |
| 00897246 | 98 | batch draw; exclusive |
| 00896E63 | 995 | clipped raster; exclusive |
| 00896053 | 110 | 4444 packed blend; exclusive helper |
| 008960C1 | 105 | 1555 packed blend; exclusive helper |
| 0089612A | 99 | 565 packed blend; exclusive helper |
| 0089618D | 105 | shared-table initializer; renderer helper |
| 00968B60 | 7 | folded/shared AddRef; no exclusive credit |
| 0091559A | 5 | folded true-return body used twice; count once, no exclusive credit |

The first twelve bodies total **1,666 bytes**; this is a bounded audited path, not a reassignment of
the inherited owner or a whole-family completeness claim. Two shared bodies add 12 referenced bytes.
Aliases of these addresses are not additional bodies. Foreign: allocator/free, `_ftol2`, target
virtual methods, the larger holder, font layout/cache/palette production, FreeType and delegated span
kernels. None receives byte credit here. No span kernel is called by the small raster body: its three
direct calls are the packed helpers above. Unknown: correspondence of these bodies to the inherited
4,444 bytes and behavior of substitute renderer/target implementations.

Identity/caller evidence bodies kept **foreign** to this inventory: drawing-director constructor
`00825B60` (115 bytes), holder class getter `00829910` (6), holder factory `0082A2B0` (27),
holder constructor `00829F70` (128), holder QI `00829920` (53), holder Release `00829960` (19),
holder unlock `00829A20` (99, forwarding branch only), and holder draw `0082A7D0`
(1,016, forwarding branch only). No full behavior claim for the latter two or the holder's other bodies.
The export's `vtable <function>` query has no slot metadata for the small object; the slot evidence
here is the original PE dword table plus the constructor and instruction dispatches, not an inferred
absence of methods from that query's zero results.

## 2. Typed slots and lifetime

`this` is the small object in ECX. All arguments below occupy 32-bit stack slots. `bool` means
the low byte AL; higher return bits need not be zero. Names are descriptive, not recovered declarations.

| Slot / offset | Target | Arguments, result and rule |
|---|---|---|
| 0 / +00 | 0089620D | QI(u32 iid, pointer-to-pointer out) → bool; accepts 1 or A8BC2A75 |
| 1 / +04 | 00968B60 | AddRef() → u32; increment +4, return new value |
| 2 / +08 | 008961F6 | Release() → u32; see below |
| 3 / +0C | 0091559A | one dword argument, ignored; true |
| 4 / +10 | 0091559A | one dword argument, ignored; true; exact semantic names unknown |
| 5 / +14 | 00896232 | Lock(font pointer, target pointer, mode) → target callback result |
| 6 / +18 | 00896243 | Unlock(font pointer, target pointer, mode) → target callback result |
| 7 / +1C | 00897246 | Draw(font pointer, target pointer, records pointer, u32 count, mode, colour-data pointer, optional clip pointer) → true |

QI success writes `this` to out, then dispatches AddRef, then returns true. Failure returns false
without clearing or writing out. No null-out check on success (`00896222–0089622D`).
Release reads +4 with a **signed** comparison: if greater than 1, decrement and return it;
otherwise free `this` directly and return zero (`008961F6–0089620C`). Thus releasing a fresh
count-zero factory object frees it. There is no member-resource destructor in this small object.
Increment/decrement are ordinary dword operations, without atomic instructions.

Lock ignores font and mode, calls target **+18(0x8080)**, and returns its AL unchanged.
Unlock similarly calls target **+1C(0x8080)**. Both pop 12 argument bytes, make no object state
change and do no null checks (`00896232–00896253`). A normal target failure propagates; a null
target dereferences. The font owns nested-lock bookkeeping, not these callbacks
([drawing-fonts.md §9](drawing-fonts.md#9-drawing-text)).

## 3. Batch dispatch and record layout

The font caller's seven pushes and call at `0089A2F3–0089A315`, together with `RET 1C` at
`008972A5`, establish slot 7's argument order. The caller maps font aaMode 2 → renderer mode 2,
3 → 3, every other value → 1 (`00896B2C–00896B44`). In mode 2 it passes the palette pointer;
otherwise it passes the address of its foreground dword. Palette generation remains the font's contract.

Each **24 hex-byte record** has:

| Offset | Value |
|---|---|
| +00, +04, +08, +0C | signed destination left, top, right, bottom |
| +10 | source coverage-byte pointer, already pointing at the glyph cell |
| +14 | source byte pitch |
| +18, +1C | source width, height (copied to temporary source rectangle right/bottom) |
| +20 | character value, tested as unsigned dword |

For nonzero count the dispatcher initializes temporary source left/top to zero. In record order,
it copies width/height, then skips character values **below 20 hex**; all others call `00896E63`
with the destination rectangle, source bytes/pitch, temporary source rectangle, mode, colour data,
and clip (`00897257–00897294`). Width/height are copied but are not used to bound the raster;
its source offset comes from source left/top and its extent comes from the clipped destination.
It advances by 24 hex bytes and decrements count to zero. Count zero accesses neither records nor
target and returns true. Negative signed counts are not rejected: the counter is a dword loop.
No per-glyph result is checked, no rollback occurs, and the batch returns true even if every glyph
is skipped or clipped away. Valid, sufficiently large source/target storage is a caller precondition.

## 4. Target access, clipping and effect order

For each eligible record, before testing emptiness, `00896E63` calls target +24 (width), +28
(height), copies both rectangles, defaults local format values to **3 and 16**, then calls target
+40 (depth). **Only mode exactly 3** calls target +38 with the local format output storage
(`00896E73–00896EBB`); its result is ignored. These are the target pointer's virtual offsets,
not offsets in the renderer. Concrete target implementations are outside this contract.

A bounded concrete target view checks these meanings independently of auto-labels. The buffer
factory `008298F0` allocates 4C bytes and calls `008272E0`, whose constructor installs **00AC1400**
at object +0. The drawing director registers that factory with getter `00825C80` (class C470D325,
`00825BBD–00825BCA`). Raw dwords in that vtable map:

| Virtual offset | Body | Concrete buffer value/effect |
|---|---|---|
| +24 | 00808620 | dword field +1C (width used by raster) |
| +28 | 004ED900 | dword field +20 (height used by raster) |
| +38 | 008268F0 | writes format field +0C and depth field +10 into two output dwords |
| +40 | 0040CE60 | depth field +10 |
| +88 | 008265C0 | pixel base field +3C |
| +8C | 0068D1B0 | byte pitch field +40 |
| +18 | 00826AA0 | false if pixel base null; otherwise OR flags into +44, increment word +38, and increment dword +30 when flag bit 15 is set; true |
| +1C | 00826490 | clear supplied flag bits from +44, decrement word +38, true |

Each getter body is four instruction bytes; their unrelated folded names do not determine target
semantics. Format output is a two-dword structure in this concrete view; `008268F0` has no boolean
return contract. It writes both values and pops four bytes. This is a pointer-view/dispatch check,
not a specification of buffer allocation or a claim that every renderer target uses this vtable.
With the renderer's 8080 flag, this target's lock increments +30 as well as +38; its unlock does
not decrement +30. Those are target effects, not state held by the 8-byte renderer.

All rectangle comparisons are signed 32-bit. With destination (L,T,R,B) and initial source
origin (Sx,Sy)=(0,0), first apply optional clip, in this order:

1. If clip.left > L, add that difference to Sx and L.
2. If clip.top > T, add that difference to Sy and T.
3. Lower R to clip.right if smaller; lower B to clip.bottom if smaller.
4. If L < 0, add −L to Sx and set L=0. Do the analogous adjustment for T and Sy.
5. Lower R to target width, B to target height. If **R ≤ L or B ≤ T**, return true without
   fetching pixel pointer/pitch or reading source/colour data.

Evidence: clip `00896EBE–00896EFF`, target bounds `00896F07–00896F38`, empty tests
`00896F3B–00896F4A`. There is no rect normalization, source-bound clipping, scaling, pen update
or stored renderer clip. Each record starts afresh. The source rectangle is copied, not mutated.

For nonempty supported depths, read the first colour-data dword, then target +8C (byte pitch),
then +88 (pixel base), in that order. Depth 16 uses words with effective byte pitch
**2·(pitch unsigned-shift-right 1)**; depths **24 and 32 both use dwords**, effective byte pitch
**4·(pitch unsigned-shift-right 2)**. Other depths silently draw nothing and return true
(`00896F50–00896F7A`, `008970D5–00897101`). In particular, depth 24 is not a packed 3-byte path.
Source first byte is base + Sy·sourcePitch + Sx. Destination first pixel is at clipped (L,T).
Rows advance by those pitches; no allocation, lock, unlock, target retention, or error recovery
occurs in the raster. Arithmetic and address calculations use 32-bit operations; this contract's
ordinary-coordinate examples assume no overflow and positive usable pitch.

The renderer receives no pen pointer. The font updates the caller's pen after batch dispatch,
then performs its automatic unlock (`0089A315–0089A348`), ignoring both callback results there.
Its automatic lock tests AL before any pen override or record generation (`0089A162–0089A183`).

## 5. Pixel modes and packing

Let a be the unsigned coverage byte, D the existing pixel, F the first colour-data dword.

| Mode | Depth 16 | Depth 24 or 32 |
|---|---|---|
| 1 | a > 127: write low 16 bits of F; otherwise preserve | a > 127: write F; otherwise preserve |
| 2 | a = 0: preserve; otherwise write low word of palette[a], with **4-byte entry stride** | a = 0: preserve; otherwise write palette[a] |
| other | nonzero a: packed blend only for local format 4,5,6,7; zero a preserves | packed dword blend for every a, including zero |

Evidence: `00896FCA–008970AC`, `0089715A–00897216`. Only mode 3 fetches format; therefore
other values outside 1/2/3 leave local format 3 and make the 16-bit blend path a no-op.
Mode 2 still reads palette[0] before pixel iteration even though zero coverage preserves.
Palette indices are raw bytes, without clamping to 127; callers must provide addressed entries.

The constructor checks byte `00B5C418`; if zero it fills **32×32×32 bytes** starting at
`00B54418`, then writes flag 1 (`0089772F–0089773D`). Indices are foreground component f,
background component d, and quantized coverage q, each 0..31. Entry index is
**(f·32+d)·32+q**, value **truncate(d·(1−q/32)+f·q/32)**. The factor is the original float
`3D000000` (1/32) at `00A8ED98`. The x87 sequence multiplies d by the complement, f by the
weight, adds, then calls `_ftol2` (`008961A1–008961D4`). These inputs are dyadic and exactly
representable; truncation is floor for these nonnegative values. The outer loops end after f=31.
No synchronization surrounds the one-time flag.

For nonzero a, q=a>>3. Components are taken from F and D; output is:

| Format | Components used in lookup | Output packing |
|---|---|---|
| 7 | bits 8..11, 4..7, 0..3 | F000 hex OR blended r<<8 OR g<<4 OR b |
| 4 or 6 | bits 10..14, 5..9, 0..4 | blended r<<10 OR g<<5 OR b; high bit cleared |
| 5 | bits 11..15, **6..10**, 0..4 | blended r<<11 OR g<<6 OR b; **bit 5 cleared** |

Evidence: entire helpers `00896053–008960C0`, `008960C1–00896129`, `0089612A–0089618C`.
Format 5 deliberately loses the low green bit in this path. Coverage 255 means 31/32, not a
full foreground replacement; coverage 1..7 has q=0 yet may still change forced alpha/unused bits.

For the dword blend, the normative packed arithmetic is given here rather than assuming four
independent channel interpolations. All operations below wrap modulo 2^32; shifts are logical.
With M=00FF00FF hex, define:

- H = ((D >> 8) AND M)·(256−a) + ((F >> 8) AND M)·a.
- J = (D AND M)·(256−a) + (F AND M)·a + 00800080 hex.
- Result = (((J >> 8) XOR (H − 007FFF80 hex)) AND M) XOR (H + 00800080 hex).

This writes a full dword, including the high byte; it does not promise conventional alpha blending.
Evidence: multiplies `00897180–008971B4`, logical shift and XOR/mask packing
`008971BB–008971D0`. No destination colour-type query controls this dword path.

## 6. Literal traces and limits

These are independent literal consequences of the cited instructions, with valid backing storage
and ordinary positive pitches. Integer calculations were checked with a local arithmetic worksheet;
no original instructions were executed. They are specification vectors, not engine tests or game verdicts.

### 6.1 Clipping, emptiness and order

| Input | Expected trace |
|---|---|
| count 0; null records and target | true; no record/target/colour/source access |
| one record, character 1F hex | copy source width/height and read character; skip raster; true, no target calls |
| same record, character 20 hex | enter raster; queries width, height, depth even if destination is empty |
| target 8×6, destination (−2,−1,7,5), clip (1,1,6,4), source pitch 12 | source origin becomes (3,2), destination (1,1,6,4), extent 5×3; source offsets 27..31, 39..43, 51..55 |
| preceding input, depth 32, target byte pitch 36 | first-pixel byte offsets 40, 76, 112 on the three destination rows; five dwords on each row |
| target 8×6, destination (8,1,10,3), no clip | right lowers to 8, hence empty; no colour read, +8C or +88 call |
| destination (2,2,4,4), clip (4,0,8,6) | left advances to 4, source x advances by 2, width becomes zero; no pixels |
| destination (5,1,3,4), no clip | reversed width remains empty; no normalization |
| nonempty destination, depth 8 | width/height/depth (plus format query only in mode 3); then true without colour read or pixel access |
| nonempty destination, depth 24, byte pitch 15 | dword pixel path, effective row stride 12 bytes; no 3-byte pixel packing |

For the 5×3 clipped vector, setting record source width/height to (1,1) changes no pixel extent;
the copied right/bottom are unused. Storage must still cover the listed source offsets.
Evidence: batch gate `00897250–00897299`, clipping/empty branch `00896EBE–00896F4A`,
source addressing `00896F80–00896FA1` / `00897107–00897122`.

### 6.2 Threshold, palette and packing

For mode 1 with F=89ABCDEF hex and any initial D, coverage **0,127,128,255** yields
**D,D,CDEF,CDEF** at depth 16, or **D,D,89ABCDEF,89ABCDEF** at depth 24/32.
For mode 2, let palette[0]=DEADBEEF, [1]=12345678, [128]=89ABCDEF, [255]=76543210:
coverage **0,1,128,255** yields **D,5678,CDEF,3210** at depth 16, or
**D,12345678,89ABCDEF,76543210** at depth 24/32. The first palette dword is read but is not
written for coverage zero; indices 128 and 255 demonstrate the absence of a 127 clamp.
The palette example presumes at least 256 readable entries; it makes no such allocation claim
for the font's own palette producer. Evidence: `00897075–008970AC`, `008971E1–00897216`.

At depth 16 in mode 3, with the format output set as listed:

| Format | D | F | a | Result | Discriminator |
|---|---|---|---:|---|---|
| 7 | 0000 | FFFF | 0 | 0000 | zero coverage skips store |
| 7 | 0000 | FFFF | 1 or 7 | F000 | q=0 but alpha forced |
| 7 | 0000 | FFFF | 255 | FEEE | greatest coverage is 31/32 |
| 4 or 6 | 8000 | 0000 | 1 | 0000 | high bit cleared even at q=0 |
| 4 or 6 | 0000 | 7FFF | 255 | 7BDE | five-bit table blend |
| 5 | 0020 | 0000 | 1 | 0000 | low green bit discarded |
| 5 | 0000 | FFFF | 255 | F79E | green lookup is bits 6..10 |

All pixel words above are hexadecimal. With mode 4 instead of 3, format is not fetched and stays
3; the same nonempty depth-16 records make no pixel changes, even at a=255.
These distinguish a renderer-specific lookup from a general span kernel's blend.

For depth 24/32 in mode 3 (packed formula of §5):

| D | F | a | Result |
|---|---|---:|---|
| 11223344 | FFEEDDCC | 0 | 11223344 (a store still occurs) |
| 11223344 | FFEEDDCC | 128 | 88888888 |
| 00000000 | FFFFFFFF | 128 | 80808080 |
| 00000000 | FFFFFFFF | 255 | FEFEFEFE |

The dword zero-coverage case reads and writes D; the 16-bit zero-coverage case skips both
the helper and store. A read-only destination is therefore not interchangeable with an untouched
pixel even when the resulting value is unchanged (`00896FF9–00897010` versus `00897180–008971D0`).

### 6.3 Lifetime, lock failures and pen

| Starting state / operation | Expected result/effect |
|---|---|
| fresh small object, count 0; QI(unknown, &sentinel) | false; sentinel and count unchanged |
| same live object; QI(A8BC2A75, &out) | out=this, count 1, true |
| count 1; AddRef, Release, Release | returns 2, then 1, then 0 and frees object |
| separate fresh count-0 object; Release | free and return 0 |
| count bit pattern FFFFFFFF; Release | signed value −1 ≤ 1, so free, return 0 |
| target +18 returns false; renderer Lock(font,target,3) | one +18(8080) call, false; no renderer count/state change |
| target +1C returns false; renderer Unlock(font,target,1) | one +1C(8080) call, false; no renderer state change |

For the concrete 00AC1400 target view, null pixel base makes Lock false before its counter/flag
changes. With pixel base nonnull, lock starts with word +38=0, dword +30=0, flags +44=0:
Lock gives word 1, dword 1, flags 8080; Unlock gives word 0, dword **1**, flags 0, true.
This follows `00826AA0–00826AC7` and `00826490–008264A6`; renderer forwarding does not
repair or supplement those target effects.

Font-side ordering trace (not additional renderer state): assume initialized font, nonempty text,
target present, font lock count 0, supplied pen (30,40), and call x/y (10,20). If automatic lock
returns false, DrawTextLine returns false before reading/writing that pen or building records,
and does not increment the lock count or unlock. Palette preparation may already have occurred.
If lock succeeds and one valid glyph has offset (0,0), size 3×3, advance 4, zero spacing,
and clip (0,0,5,5), it emits no record but advances the pen to **(34,40)**, then unlocks.
If an emitted batch instead returns false from a substitute renderer, the font still writes its
advanced pen and unlocks; it ignores the batch AL. Sources: `0089A162–0089A186`,
`0089A253–0089A2BF`, `0089A315–0089A34B`. The small renderer itself always returns true
from the batch on normal completion; a fault has no promised normal-return result or rollback.

Unresolved boundaries are filed in [questions/font-renderer.md](questions/font-renderer.md).
This contract does not establish native equivalence, renderer replacement eligibility or completion
of the larger labelled owner. Arbitrary invalid pointers, overflowing geometry, negative/zero pitch,
concurrency, and foreign target fault behavior have no safe equivalence promise here.
