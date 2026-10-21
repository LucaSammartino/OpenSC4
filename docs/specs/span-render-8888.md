# Span renderer A8R8G8B8 (`nGZDrawSoftRender::cSpanRender8888`)

Status: RE spec, lane re-span8888-4, 2026-10-21. Source: SimCity 4 Deluxe GOG 1.1.641 Windows image, Ghidra export
(`re_query.py asm`) and raw table reads from the image. Every numeric rule was read from the instructions at the
cited addresses and then checked by executing the image bytes against the rules of this file (section 10). Nothing
is copied from the 1555/4444 specs without that check; where the rule happens to be the same it is restated here.

Scope: the owner range `0x00854E10`-`0x0085E640` (174 functions, 37,218 bytes, `tools/coverage-ranges.tsv:22`): the
five vtable methods, all 168 blit kernels of its `RenderBlt` table (every table entry lies in this range, none is
folded into another owner) and one blend helper `0x008576B0`. Out of scope: `cGZDrawContextRenderToBuffer` itself
(view3d.md section 7), the other span renderers, the CRT helper `memset32` `0x00910003` (described only as used).

## 1. Class and creation

- vtable `0x00AC1874`, 6 slots: 0 deleting destructor `0x0082FF00` (shared by all span renderers), 1 `RenderSolid`
  `0x00854E10`, 2 `RenderGradient` `0x00854E60`, 3 `RenderAlpha` `0x00854F70`, 4 `RenderAlphaGradient`
  `0x00855040`, 5 `RenderBlt` `0x008551D0` (`re/symbols/gzdraw-softrender.tsv`).
- Created by `cGZDrawContextRenderToBuffer::Init` (`0x00830710`) for buffer colour types 8 (R8G8B8, i.e. X8R8G8B8
  in 32 bits) and 9 (A8R8G8B8). The object has no data members the methods read: `this` (ECX) is never
  dereferenced in slots 1-5. Slots 1-5 are `thiscall` with callee-cleaned stacks (`RET 0x10`, `0x10`, `0x14`, `0x10`,
  `0x18`); all five preserve EBX, ESI, EDI, EBP.

### 1.1 Native call boundary (re-span8888-bridge4) [exact]

Read from the bytes at the cited addresses for this class (`re_query.py asm`, raw dword reads of the image). The
shared context code (`Init`, `DrawPreclippedSpanList`, factory) is the code `span-render-1555.md` §1.1 and
`span-render-4444.md` §1.1 cite; it was re-read here for the 8888 case.

**Vtable.** `0x00AC1874` holds exactly `0x0082FF00 0x00854E10 0x00854E60 0x00854F70 0x00855040 0x008551D0`. Each
of the five method addresses occurs exactly once in the image as a dword (its own entry `0x00AC1878`…`0x00AC1888`),
and the immediate `0x00AC1874` occurs exactly once, in the Init store at `0x008307CC`. Patching entries 1-5
therefore redirects every type-8 and type-9 RenderToBuffer context; nothing copies the table. The kernel row array
`0x00AC23D8` is referenced only by RenderBlt (`0x008551E2`).

| Slot | Offset | Method | Arguments (dwords, pushed right to left) | Callee pops (address of `RET`) | Saved registers |
|---|---|---|---|---|---|
| 0 | +0x00 | deleting destructor `0x0082FF00` | `flags` | `RET 4` (`0x0082FF1C`) | ESI |
| 1 | +0x04 | RenderSolid `0x00854E10` | `spans, n, surface, colour` | `RET 0x10` (`0x00854E53`) | EBX EBP ESI EDI |
| 2 | +0x08 | RenderGradient `0x00854E60` | `spans, n, surface, grad` | `RET 0x10` (`0x00854F64`) | EBX EBP ESI EDI |
| 3 | +0x0C | RenderAlpha `0x00854F70` | `spans, n, surface, colour, alpha` | `RET 0x14` (`0x00855035`) | EBX EBP ESI EDI |
| 4 | +0x10 | RenderAlphaGradient `0x00855040` | `spans, n, surface, grad` | `RET 0x10` (`0x008551CB`) | EBX EBP ESI EDI |
| 5 | +0x14 | RenderBlt `0x008551D0` | `spans, n, surface, src, b, kind` | `RET 0x18` (`0x0085525F`) | ESI EDI |

MSVC thiscall: `this` in ECX and never read (the first use of ECX in every method is a write: `0x00854E26`,
`0x00854E63`, `0x00854F92`, `0x0085504C`, `0x008551D5`), no return value (no caller reads EAX after these calls;
`DrawPreclippedSpanList` returns right after each). `spans`, `surface`, `grad`, `src` (slot 5) and `b` are
pointers; `n`, `colour`, `alpha` and `kind` are values. RenderSolid's helper `memset32` `0x00910003` is cdecl
`(dest, value, count)`, popped by the caller (`ADD ESP,0xC` at `0x00854E49`). RenderBlt indexes the row array with
`b.byte(+0x1C) + 2 × b.dword(+0x10)` (`0x008551D5`-`0x008551DC`), copies `b` (32 bytes), `src` (24) and `surface`
(24) by value, calls `rows[…][kind]` (cdecl, 22 dwords, `CALL [EAX+ECX*4]` at `0x00855257`) and pops them itself
(`ADD ESP,0x58` at `0x0085525A`). Slot 0 is thiscall `(flags)`: it stores the base vtable `0x00AC17AC`, frees
`this` with `GlobalHeapFree` `0x005E5620` when bit 0 is set, returns `this` in EAX.

**Creation (factory selection).** `cGZDrawContextRenderToBuffer::Init` (`0x00830710`, thiscall `(buffer)`,
`RET 4` at `0x00830877`/`0x00830892`/`0x008308AB`, returns AL): calls its own `Shutdown` (vfunc `+0x10`, which
destroys any previous renderer and clears `+0xB0`), releases the buffer held at `+0x08`, QueryInterfaces `buffer`
for `cIGZBuffer` `0x20732180` into `+0x08` (failure → return 0), locks it (`vfunc +0x18`, flags `0x8040`;
failure → release, return 0) and switches on `GetColorType` (`vfunc +0x3C`, first dword) minus 2 through the
table at `0x008308B0` = `008307D4 0083087A 0083077C 00830792 0083077C 008307A8 008307BE 008307BE`. Types **8 and 9
share case `0x008307BE`**: `GlobalHeapAlloc(4)` (`0x005E55E0`, call `0x008307C0`), store `0x00AC1874` at
`0x008307CC`, pointer stored at `ctx+0xB0` (`0x008307EE`). A failed allocation stores 0 and Init still returns 1;
the four slot loads in `DrawPreclippedSpanList` (`0x008304AD`, `0x00830529`, `0x00830583`, `0x008305C2`) do not
test it, so the next draw would fault *[compat: an implementation may fail Init instead]*. Types 3, 1, 0 and
anything above 9 release the buffer and return 0 (`0x0083087A`). On success Init caches `ctx+0x78` = pixel
pointer (`vfunc +0x88`), `+0x7C` = stride (`+0x8C`), `+0x80` = width (`+0x24`), `+0x84` = height (`+0x28`),
`+0x88` = colour type, `+0x8C`/`+0x90` = `vfunc +0x44` (bytes per pixel), clip `+0x0C`/`+0x10` = width/height,
clears `+0xA4` (tiling), sets `+0x75` = 1, returns 1. The buffer stays locked until `Shutdown`/`Release`. The
renderer never reads the colour type: type 8 and type 9 targets are drawn identically (the alpha byte written is
per operation, §2-§9), and every method assumes 4 bytes per pixel and the stride the buffer reported.

**Lifetime.** One renderer per Init; `Shutdown` (`0x0082FF20`) and `Release` (`0x008306C0`) call slot 0 with flags
1 and clear `+0xB0`. The object is 4 bytes (only the vtable pointer) and holds no state, so a successor may
share one stateless instance (nothing observes the identity).

**The only native caller** of slots 1-5 is `DrawPreclippedSpanList` (`0x008302E0`, thiscall `(spans, n)`, `RET 8`,
slot 22 of the context vtable `0x00AC17D0`). It passes its own `spans`, `n` unchanged and `surface = ctx+0x78`
always, and calls exactly one slot per call:

| Condition (context fields) | Slot, call site | Extra arguments |
|---|---|---|
| texture `+0x24` set, lock (`vfunc +0x18`, `0x800`, `0x008302FB`) succeeds, colour type mapped | 5 at `0x008304D1` / `0x0083053B` / `0x00830591` (tiled / plain / stretched set-ups) | `src` = 24-byte texture record on the stack, `b` = 32-byte blit record, `kind` |
| no texture, gradient `+0x74`, alpha `+0x41` | 4 at `0x008305E7` | `grad = ctx+0x44` |
| no texture, gradient, no alpha | 2 at `0x008305FC` | `grad = ctx+0x44` |
| no texture, no gradient, alpha | 3 at `0x00830627` | `colour = [ctx+0xC4]`, `alpha = [ctx+0xB4]` |
| no texture, no gradient, no alpha | 1 at `0x00830647` | `colour = [ctx+0xC4]` |

Texture path (the rules of `span-render-1555.md` §6.1, re-read here for the values this class sees): source
format `b.+0x10` from the texture colour type through the table `0x00830654`: 7 → 0, 6 → 1, 4 → 2, 5 → 3,
**9 → 4, 8 → 5**, 2 → 6; type 3 or outside 2..9 → unlock, nothing drawn (`0x008305AA`). Key flag `b.+0x1C` = the
byte `vfunc +0x68` returns, key into `b.+0x18`, then masked by format through `0x00830674`: 0 `& 0x0FFF`, 1 and
2 `& 0x7FFF`, 3 `& 0xFFFF`, 4 and 5 `& 0x00FFFFFF`, 6 → flag forced to 0 (`0x008303FF`), so **row 13 (keyed A8) is
unreachable** from this caller. For a `cGZBuffer` texture `vfunc +0x68` is `0x00826620`: returns 1 and the key
`+0x24` if bit 1 of `+0x28` is set, else 0, so the flag is 0 or 1. `b.+0x14` = `[ctx+0xB4]`; `kind` = alpha
`+0x41` + 2 × modulate `+0x42`, `+8` if tiling `+0xA4`, else `+4` if stretching `+0x40`. The texture is unlocked
(`vfunc +0x1C`) after the call.

**Supported COM boundary.** No interface slot returns or sets `+0xB0`. A RenderToBuffer context is made only by
`cIGZDrawSystem` slot 4 (`0x0082BAB0`, entry `0x00AC1678`, thiscall `(buffer, out)`, `RET 8`), which asks each
registered factory's slot 3 in list order and stops at the first success. The RenderToBuffer factory
(`0x0082B910`) accepts any object answering `cIGZBuffer` `0x20732180`, allocates 0xD4 bytes, constructs
(`0x00830690`) and AddRefs into `*out` (a failed allocation faults at `0x0082B950`); it does not call Init. The
native producer of that call is `cGZDrawContext` slot 17 (`0x0082C830`, entry `0x00AC170C`, the front-end
draw context's set-target method): it releases the previous engine (`+0x10` then Release), AddRefs the new
target into `+0x24`, copies its area (`target vfunc +0x30`) to `+0x34`, gets the draw system (IID `0x4B2EA3B3`,
`0x0082C8CC`), calls slot 4 `(target, &ctx+0x28)` (`0x0082C8F6`), then calls the engine's `Init(target)`
(`+0x0C`, `0x0082C907`) and pushes its state to it (`+0x18` clip `&ctx+0x34`, `+0x1C`, `+0x20`, `+0x24`, …).
Neither slot 4's nor Init's result is tested there.

**Original-vs-ours trace (one solid span, constructed buffer).** Original: `cGZBuffer` (CLSID `0xC470D325`)
`Init`-ed with (9, 32) → `cGZDrawContext` set-target `0x0082C830` → draw system slot 4 → factory `0x0082B910`
→ context `Init` → case `0x008307BE`, `+0xB0` = 8888 renderer → a fill through the front end reaches
`DrawPreclippedSpanList` → slot 1 `0x00854E10(spans, 1, ctx+0x78, colour)` → `memset32`. Ours: the same span list,
`n`, surface record (base, stride) and colour into the RenderSolid rule of §3. Example: `spans =
[{0x00010002, 0x00010005}]`, surface `{B, 16}`, colour `0x12345678`: dwords at `B+24`, `B+28`, `B+32` become
`0x12345678`, nothing else changes, 16 bytes popped.

## 2. Shared conventions

**Pixel.** 32 bits, A in bits 24-31, R 16-23, G 8-15, B 0-7. All writes are whole dwords. The renderer treats
types 8 and 9 the same: what it writes to the alpha byte is stated per operation (often a constant `0xFF`).

**Span record** (8 bytes): dwords `start`, `end`, each `(y << 16) | x` (unsigned 16-bit x, y). Width =
`end - start` on the whole dwords (exclusive end, one row per span).

**Target descriptor** (`buffer`): dword `+0` base of pixel (0,0), `+4` row stride in bytes. Pixel (x, y) is the
dword at `base + y*stride + 4*x`. No clipping and no bounds test anywhere in this class: spans are pre-clipped by the
caller (`DrawPreclippedSpanList`, view3d.md 7.1). The stride is used as a signed 32-bit multiplier of y
(`IMUL`), so negative strides address upward; only non-negative strides were executed (U4).

**Span list loop.** Every method and kernel takes `(spans, count, ...)` and processes the `count` spans in order
with a decrement-then-test loop, so `count` must be at least 1 (count 0 is not guarded, U1). A span of width 0
writes nothing (RenderSolid passes 0 to `memset32`, a `REP STOSD`, which stores nothing; every other loop tests
the width first).

**The blend rule `B(d, s, w)`** (used by RenderAlpha, RenderAlphaGradient and the alpha blit ops). For each of R,
G, B, with 8-bit `d` from the destination and `s` from the source colour:
`out = floor((256*d + w*(s - d) + 128) / 256)`, and the output alpha byte is always `0xFF`. The code computes R and
B together in one register (`((sRB - dRB)*w + 0x00800080) >> 8`, then `+ dRB`, masks `0x00FF00FF`) and G alone;
the borrow that a negative low field (B) makes into the high field (R) is cancelled exactly by the carry of the
following `+ dRB`, so for `0 <= w <= 256` the packed result equals the per-channel formula above for every input
(executed, section 10). The source alpha byte is ignored by `B`. Worked example: d = `0x12345678`,
s = `0x00FF0000`, w = 128: R = floor((256*0x34 + 128*(255-52) + 128)/256) = floor((13312 + 25984 + 128)/256) = 154
= `0x9A`; G = floor((22016 - 11008 + 128)/256) = 43 = `0x2B`; B = floor((30720 - 15360 + 128)/256) = 60 = `0x3C`;
result `0xFF9A2B3C`.

## 3. RenderSolid (`0x00854E10`)

Arguments `(spans, count, buffer, colour)`. Each span: `memset32(pixel(x, y), colour, width)` - the 32-bit
`colour` is stored verbatim, alpha byte included.

## 4. RenderAlpha (`0x00854F70`) - constant colour over the destination

Arguments `(spans, count, buffer, colour, alpha)`.
- Weight `w = (alpha >> 24) + (alpha >> 31)` (unsigned): the alpha byte `A` plus its top bit, so A = 0 gives 0,
  `0x7F` gives 127, `0x80` gives 129, `0xFF` gives 256. Only the top byte of `alpha` matters.
- Each pixel becomes `B(d, colour, w)` (section 2); the alpha byte of `colour` is ignored, output alpha `0xFF`.
- So A = `0xFF` reproduces `colour`'s RGB exactly (w = 256), A = 0 leaves RGB unchanged but still forces the
  destination alpha byte to `0xFF`.

## 5. RenderGradient (`0x00854E60`) - linear colour ramp, opaque

Arguments `(spans, count, buffer, grad)`; `grad` points to dwords (signed 8.16 fixed point, bits 16-23 are the
8-bit channel): `+0x00` B0, `+0x04` G0, `+0x08` R0, `+0x10` dB/dx, `+0x14` dG/dx, `+0x18` dR/dx, `+0x20` dB/dy,
`+0x24` dG/dy, `+0x28` dR/dy (`+0x0C`, `+0x1C`, `+0x2C` and anything beyond are not read).
- Per span, with (x, y) the span start: `accB = B0 + x*dB/dx + y*dB/dy`, the same for G and R (32-bit wrap), then
  `accR |= 0xFF000000`.
- Each pixel writes `(accR & 0xFFFF0000) | (byte2(accG) << 8) | byte2(accB)` where `byte2(v) = (v >> 16) & 0xFF`;
  then each accumulator adds its x step. There is no clamping: a channel that runs past 255 or below 0 wraps
  (only its bits 16-23 are kept).
- The alpha byte is the top byte of the running `accR`: `0xFF` at the span start, but the x steps are added to the
  whole dword, so a carry out of bit 23 (R running past 255) or a borrow (R running below 0) changes it (for example
  to `0x00` or `0xFE`). With R in 0..255 for the whole span the alpha byte stays `0xFF`.
- Worked example: span at x = 2, y = 1, B0 = `0x00100000`, dB/dx = `0x00010000`, dB/dy = `0x00020000`, others 0:
  accB starts at `0x00100000 + 2*0x10000 + 0x20000 = 0x00140000`, pixels `0xFF000014`, `0xFF000015`, ...

## 6. RenderAlphaGradient (`0x00855040`) - colour and alpha ramp over the destination

Arguments `(spans, count, buffer, grad)`; `grad` as in section 5 plus a fourth channel: `+0x0C` W0,
`+0x1C` dW/dx, `+0x2C` dW/dy (12 dwords read).
- Accumulators as in section 5 (the code adds `0xFF000000` to accR, which has no effect on the output).
- Per pixel: source colour `s = byte2(accR) << 16 | byte2(accG) << 8 | byte2(accB)`, weight `w = accW >> 16`
  (unsigned, the whole top half, 0..65535: it is not masked to a byte and not clamped). The pixel becomes
  `B(d, s, w)` for `w <= 256`, output alpha `0xFF`. Then all four accumulators add their x step.
- For `w > 256` the packed computation is still exactly defined and is what the program does (executed with random
  w up to 65535, section 10): with `sRB = s & 0x00FF00FF`, `dRB = d & 0x00FF00FF`, `sG = s & 0xFF00`,
  `dG = d & 0xFF00`, all arithmetic modulo 2^32 and `>>` logical:
  `RB = (((sRB - dRB)*w + 0x00800080) >> 8) + dRB`, `G = (((sG - dG) >> 8)*w + dG + 0x80)`,
  `out = (RB | 0xFF00FF00) & (G | 0xFFFF00FF)`. The fields then interfere (garbage colour, alpha still `0xFF`).
  The intended range is `accW` in `0 .. 256 << 16` (an alpha of 1.0 = `0x01000000`).

## 7. RenderBlt (`0x008551D0`) - kernel dispatch

Arguments `(p1, p2, rectA, rectB, src, mode)`, identical in shape to the other span renderers: `rectA`, `rectB`
point to 24-byte records, `src` to a 32-byte source descriptor. It copies the three records by value and calls
`kernel(p1, p2, *rectA, *rectB, *src)` (cdecl, 88 bytes of arguments), with
`kernel = table[src.format * 2 + src.keyed][mode]`, `src.format` the dword at `src+0x10` (0..6) and `src.keyed`
the byte at `src+0x1C` (0/1). The row-pointer array is at `0x00AC23D8` (14 dwords); row `r` is the 12-entry array
at `0x00AC23A8 - 0x30*r`, so the 168 entries occupy `0x00AC2138`-`0x00AC23D7`. Neither index is range-checked
(U2). 168 distinct kernels, all in this owner range.

Kernel arguments as the kernels read them:
- `p1` = span list, `p2` = span count (section 2 loop).
- `rectA` (24 bytes) = destination descriptor: `+0` base, `+4` stride. `+8..+0x14` unread.
- `rectB` (24 bytes) = source image: `+0` base, `+4` stride (bytes), `+8` width, `+0x0C` height (the last two
  only by tiled kernels). `+0x10`, `+0x14` unread.
- `src` (32 bytes): `+0x00` u0, `+0x04` v0, `+0x08` du/dx, `+0x0C` dv/dy, `+0x10` format, `+0x14` constant colour
  `K` (A8R8G8B8), `+0x18` colour key, `+0x1C` keyed flag byte; bytes `+0x1D..+0x1F` unread. Kernels use the
  argument slots as scratch (the caller's copies are by value, so nothing visible changes).

## 8. Kernel table: rows, columns, conversions

**Rows** `r = format*2 + keyed`. **Columns** `mode = 4*geometry + op`: geometry 0 direct, 1 stretch, 2 tiled;
op 0 copy, 1 constant alpha, 2 modulate, 3 modulate + alpha (the 12 Mac `cBlitter` methods `Blt`,
`BltConstantAlpha`, `BltModulate`, `BltModulateAlpha` and their `Stretch`/`Tiled` forms). The helper `0x008576B0`
(`B(d, s, w)`, cdecl `(d, s, w)`) is called by exactly the 84 kernels of columns 1, 3, 5, 7, 9, 11.

**Source formats and the widening to 8888 `C(p)`** (every op starts from `C`; read at the column-0 kernels
`0x00855270`, `0x008569F0`, `0x00858210`, `0x00859850`, `0x0085B030`, `0x0085C1E0`, `0x0085D470`):

| format | source pixel (bytes) | `C(p)` |
|---|---|---|
| 0 | A4R4G4B4 (2) | each nibble `n` -> `n*17` (`0x1` -> `0x11`), alpha included |
| 1 | A1R5G5B5 (2) | A = `0xFF` if bit 15 else 0; 5-bit `v` -> `(v << 3) \| (v >> 2)` |
| 2 | X1R5G5B5 (2) | as format 1 with A = `0xFF` (bit 15 ignored) |
| 3 | R5G6B5 (2) | A = `0xFF`; R, B as 5-bit above; 6-bit `g` -> `(g << 2) \| (g >> 4)` |
| 4 | A8R8G8B8 (4) | unchanged |
| 5 | X8R8G8B8 (4) | `(p & 0x00FFFFFF) \| 0xFF000000` |
| 6 | A8 (1) | `p << 24 \| 0x00FFFFFF` (white with the byte as alpha) |

So the 8888 renderer never truncates: `C` is the exact bit-replicated widening.

## 9. Kernel semantics

### 9.1 Geometry (source texel for destination pixel (x, y))
- Direct: source pixel (x, y) of `rectB` (the caller pre-offsets `rectB.base`); `u0`, `v0`, `du`, `dv` unread.
- Stretch: per span `v = (dv*y + v0) >> 16` (32-bit wrap, unsigned shift), row `base + v*stride`;
  `u = du*x + u0`; each pixel reads texel `u >> 16` (unsigned) then `u += du`. x, y are absolute destination
  coordinates; no clamping, no filtering.
- Tiled: per span `tx = (x + u0) mod width`, `ty = (y + v0) mod height` (unsigned 32-bit `DIV`; u0, v0 are integer
  texel offsets here); each pixel reads (tx, ty), then `tx += 1`, wrapping to 0 on reaching `width`. `du`, `dv`
  unread.

### 9.2 Operations (K = the dword at `src+0x14`)
- **Copy** (op 0): write `C(p)`. The destination is not read; the alpha byte comes from `C` (so formats 0, 1, 4, 6
  carry the source alpha into the target, formats 2, 3, 5 write `0xFF`).
- **Constant alpha** (op 1): `w = K >> 24` (0..255), pixel = `B(d, C(p), w)`, output alpha `0xFF`. The weight
  never reaches 256, so K alpha `0xFF` does not reproduce the source exactly: s = 255 over d = 0 gives
  floor((255*255 + 128)/256) = 254. (Contrast RenderAlpha, whose weight reaches 256.)
- **Modulate** (op 2): per byte channel including alpha, `M = (C * K + 128) >> 8` with `K`'s matching byte (MMX
  word multiply, rounding constant `0x0080` per word from `0x00AC1B80`); write M. The destination is not read.
  Worked example: C = `0xFFFFFFFF`, K = `0x80808080` gives `0x80808080`; C channel `0x10`, K `0xFF` gives
  floor((16*255 + 128)/256) = 16.
- **Modulate + alpha** (op 3): M as op 2, then pixel = `B(d, M, M >> 24)`, output alpha `0xFF`.
- **Keyed rows** (odd r): before anything else, a texel with `(p & mask) == key` (key = dword `src+0x18`) leaves
  the destination pixel untouched. `mask` by format: 4444 `0x0FFF`, 1555 and X555 `0x7FFF`, 565 `0xFFFF`, 8888 and
  X888 `0x00FFFFFF` (`0x0085B890`, `0x0085CAE0`). The comparison is on the raw texel, before widening. A8 has no
  colour bits: the kernels test `key == 0` once, so with key 0 nothing is drawn and with any other key every texel
  is drawn (`0x0085DD40`).

## 10. Verification

Method (scripts in `docs/specs/proofs/span-render-8888/`, they read `bin/SimCity4.exe`; nothing else is needed):
`emu.py` is a small x86-32 interpreter (integer ALU with ZF/CF, `DIV`, `IMUL`, `REP STOSD`, the MMX subset
`MOVD/MOVQ/PUNPCKLBW/PXOR/PMULLW/PADDW/PSRLW/PACKUSWB/EMMS`); an unknown instruction aborts the trial and counts as
a failure, so every instruction the runs reached is modelled. The image bytes are decoded with Capstone, not taken
from Ghidra. `check.py` drives all 168 kernels with the 88-byte RenderBlt argument block, `meth.py` the four span
methods; each compares every pixel of every span plus the untouched pixel either side with the rules above, and
checks ESP and EBX/ESI/EDI/EBP on return. Inputs: 1-4 spans of 1-25 pixels (methods: 0-25), random destination
pixels (10 % all-zero or all-one), random texels, K random or one of `0xFFFFFFFF, 0, 0x80808080, 0xF0FFFFFF`, keys
equal to a masked texel 80 % of the time (A8: key 0 half the time), random entry registers, random values in every
field called unread, images 5x3 / 37x11 / 64x16 with row padding, stretch u0 < 2.0 and steps < 1 texel, tiled
offsets 0..999.

### 10.1 Runs

| run | command | result |
|---|---|---|
| all kernels, random | `check.py 150` | 168/168 kernels, 747,927 pixels, 0 mismatches, registers and ESP preserved |
| texel sweep | `EXH=1 check.py 1 <25 kernels>` | every 16-bit texel (65,536) and every A8 byte, columns 0-3 of rows 0, 2, 4, 6, 12 and column 0 of keyed rows 1, 3, 5, 7, 13: 25 kernels, 1,312,000 pixels, 0 mismatches |
| span methods | `meth.py 300` | solid 9,551 px, alpha 9,275, gradient 8,617, alpha gradient 9,298: 0 mismatches |
| alpha gradient, w > 256 | `BIGW=1 meth.py 200` | 6,323 px, 0 mismatches against the packed formula of section 6 |
| blend forms | `blendproof.py` (numpy) | 2,425,356,288 (d, s, w) cases, 0 mismatches for both packed forms |

`blendproof.py` closes the gap that random execution leaves in the arithmetic: it transcribes the two packed blend
sequences (RenderAlpha/RenderAlphaGradient, and the helper `0x008576B0`, which extracts G by an XOR of two packed
sums instead of a separate multiply) into word arithmetic and compares them with `B(d, s, w)` over w = 0..256,
every (s, d) pair of each channel and 48 fixed pairs of the other two channels with both signs of `s - d`. The
transcriptions themselves are confirmed against the image bytes by the execution runs above.

Sensitivity: one changed model rule fails at once (`check.py 6`, `meth.py 20`): blend rounding `+127` 77 kernels;
constant-alpha weight `w+1` 40; modulate without `+128` 79; 4444 key mask `0xFFFF` 10 of 12 keyed 4444 kernels;
tiled start without `u0` 52; X888 alpha `0xFE` 18 of 24; RenderAlpha weight `+1`, RenderGradient without the
`0xFF000000`, RenderAlphaGradient weight `+1`: 375, 648, 409 mismatching pixels.

So every rule of sections 2-9 holds on the executed bytes of all 174 functions for the input ranges of section 10;
what is not executed is listed in U4.

## 11. Unknowns
- U1. `count = 0` (decrement-then-test loops would run 2^32 times): callers always pass >= 1? Not checked here.
- U2. `src.format > 6` or `mode > 11` index past the table; callers are assumed to pass valid values.
- U3. Caller meaning of `rectA/rectB +0x08..+0x14` beyond what kernels read (view3d.md owns the caller).
- U4. Input ranges not executed: negative strides, spans wider than 25 pixels, stretch steps >= 1 texel or
  accumulator wrap, tiled `x + u0` overflow, width or height 0 (tiled `DIV` by zero would fault).

## 12. Coverage proposal
Specified: the whole owner range `0x00854E10`-`0x0085E640` (174 functions): 5 methods, 168 kernels, helper
`0x008576B0`. Proposed verification level: executed (section 10).
