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
class getter `00829910` returns **8B3255F9**. Its QI `00829920` also accepts A8BC2A75 and 1;
sharing an interface does not make these the same object. Its Release `00829960` decrements to
zero then calls its own deleting-destructor slot +20, unlike the small object's direct free.
This establishes the separate pointer views, without specifying that holder's drawing/cache system.
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

Further discriminating traces and unresolved boundaries are recorded with this section and in
[questions/font-renderer.md](questions/font-renderer.md). This contract does not establish native
equivalence, renderer replacement eligibility or completion of the larger labelled owner.
