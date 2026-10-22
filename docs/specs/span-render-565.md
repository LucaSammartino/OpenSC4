# Span renderer, 16-bit 565 (`nGZDrawSoftRender::cSpanRender565`)

Owner range `0x0083DE50`–`0x00847CE0` (145 functions, 39 051 bytes; `docs/specs/view3d.md` §7.2): the five
methods of vtable `0x00AC1844`, the blend helper `0x00845450`, and 139 blitter bodies. This is the span back end
that `cGZDrawContextRenderToBuffer::Init` (`0x00830710`) creates into `+0xB0` for buffer colour type **5
(`R5G6B5`)** only (Init's jump table `0x008308B0`: type 5 → `0x00830792`, which stores vtable `0x00AC1844`). The
1555, 4444, 8888 and 8-bit siblings have their own specs; nothing below is copied from them without being
re-derived from the 565 bytes and re-checked by the proofs of §8.

Status legend: **[exact]** = every instruction read and restated; **[verified]** = checked against the shipped bytes
by execution under an x86 emulator (§8). Every function of the owner range is [exact] or [verified].

Pixel layout (destination word, little-endian `u16`): bits 0–4 **B**, 5–10 **G** (6 bits), 11–15 **R**. There is
**no alpha bit**: nothing in this class reads or keeps destination alpha, and source alpha only ever acts as a blend
weight (§7.4). Masks used by the code: `0x001F`, `0x07E0`, `0xF800`, the pair `0xF81F` (R+B) and `0x07E0` (G).

## 1. Common inputs [exact]

- **Span list.** `n` records of 8 bytes: `+0` start, `+4` end, each a packed point (x low 16 bits, y high 16 bits,
  unsigned). The pixel count is `end − start` as a 32-bit difference, so a span is valid only on one row with
  `end.x ≥ start.x`; pixels `start.x … end.x − 1` are written left to right. A zero-length span writes nothing.
- **Count `n`.** Every method and every body loops `do … while (--n)`: `n = 0` is **not** a no-op (it wraps and
  walks 2³² records). *[compat: callers never pass 0; the original's behaviour is a crash]*
- **Destination record** (24 bytes; the methods take a pointer, the bodies a copy): `+0` pointer to pixel (0, 0),
  `+4` row stride in bytes (`y × stride` is a 32-bit multiply, so a negative stride works), `+8` width, `+0xC`
  height, `+0x10` colour type, `+0x14` bytes per pixel. Only `+0` and `+4` are read here: **no clipping and no
  bounds check** happen in this class (spans arrive pre-clipped from `DrawPreclippedSpanList`, `0x008302E0`).
  Pixel address = `base + y × stride + 2 × x`.
- **Order and overlap.** Spans are processed in list order, each fully before the next, pixels in ascending x.
  Each pixel is read (source texel, then destination where blending) and written before the next pixel is touched,
  so a pixel covered twice is processed twice, and a source that overlaps the destination behaves exactly like an
  ascending element-by-element copy (a texel already overwritten earlier in the same call is read in its new
  value). Proven for all 168 blit slots with aliased buffers (§8, `span565_alias.py`).

### 1.1 Native call boundary (re-span565-bridge4) [exact]

Every address and constant below was read from the shipped bytes (raw dwords and disassembly of this image); the
context code is shared with the siblings and was re-read here for the 565 case, not copied.

**Compiled vtable `0x00AC1844`** (6 dwords; the next dword `0x0082FF00` is slot 0 of the 4444 table `0x00AC185C`):

| Slot | Entry | Target | Method | Arguments (dwords, pushed right to left) | Callee pops (RET at) | Returns |
|---|---|---|---|---|---|---|
| 0 | `0x00AC1844` | `0x0082FF00` | deleting destructor (shared by all five span tables) | `flags` | 4 (`0x0082FF1C`) | `this` |
| 1 | `0x00AC1848` | `0x0083DE50` | RenderSolid | `spans, n, dst*, colour` | 0x10 (`0x0083DE93`) | nothing |
| 2 | `0x00AC184C` | `0x0083DEA0` | RenderGradient | `spans, n, dst*, grad*` | 0x10 (`0x0083DFA5`) | nothing |
| 3 | `0x00AC1850` | `0x0083DFB0` | RenderAlpha | `spans, n, dst*, colour, argb` | 0x14 (`0x0083E06D`) | nothing |
| 4 | `0x00AC1854` | `0x0083E070` | RenderAlphaGradient | `spans, n, dst*, grad*` | 0x10 (`0x0083E20C`) | nothing |
| 5 | `0x00AC1858` | `0x0083E210` | RenderBlt | `spans, n, dst*, src*, b*, kind` | 0x18 (`0x0083E29F`) | nothing |

`*` = pointer; the others are values. Each method has exactly one `RET`. Each of the five method addresses occurs
exactly once in the image as a dword (its own entry), and the immediate `0x00AC1844` occurs exactly once (the store
at `0x008307A0`): the table is static, never copied, and patching entries 1–5 redirects every 565 context.

**Calling convention.** MSVC thiscall. `this` (ECX) is never read: every method overwrites ECX before any use (the
`LEA ECX,[ECX]` at `0x0083DFDD` is alignment padding with no effect). The object has no state. EBX, EBP, ESI, EDI
are preserved (pushed and popped). No caller reads EAX after slots 1–5. RenderSolid calls the fill helper
`0x0090FFC8` cdecl `(dest, colour, count)` and pops it itself (`ADD ESP,0xC`, `0x0083DE89`). RenderBlt reads
`b+0x10`/`b+0x1C` first, copies `b` (32 bytes), `src` (24), `dst` (24) onto its stack, pushes `n`, `spans`, calls
`rows[2·f + key][kind]` (the only other call it makes, `0x0083E297`) and pops 0x58 bytes itself (`0x0083E29A`).

**Object and lifetime.** 4 bytes holding only the vtable pointer. Created only by
`cGZDrawContextRenderToBuffer::Init` (`0x00830710`, thiscall `(buffer)`, `RET 4`, returns a byte), whose only
reference is entry `0x00AC17DC` (slot 3) of the context vtable `0x00AC17D0`. In order, Init:

1. calls its own slot 4 (`Shutdown`, `0x0082FF20`, entry `0x00AC17E0`) and releases `ctx+8` if still set;
2. queries `buffer` for `cIGZBuffer` `0x20732180` into `ctx+8` (QueryInterface adds a reference); failure → return 0;
3. locks it (`vfunc +0x18`, flags `0x8040`); failure → release `ctx+8`, clear it, return 0;
4. reads the colour type (`vfunc +0x3C`, first dword of the type/bpp pair) and jumps through `0x008308B0` on
   `type − 2` (8 entries; type 5 → entry 3 = `0x00830792`; types 3 and > 9 or < 2 → `0x0083087A`);
5. type 5: `GlobalHeapAlloc(4)` (`0x00830794`), store `0x00AC1844` (`0x008307A0`); a null allocation stores 0
   (`0x008307EA`) and Init continues and returns 1 — nothing tests `ctx+0xB0` before the next draw dereferences it;
6. stores the renderer at `ctx+0xB0` (`0x008307EE`) and caches the destination (below), sets `ctx+0xA4 = 0`, `ctx+0x75 = 1`,
   returns 1 (`0x00830866`).

The type-3/other path (`0x0083087A`) releases `ctx+8`, clears it and returns 0 **without unlocking** (see SR565-6).

Destruction: `Shutdown` calls slot 0 with flags 1 and clears `+0xB0`, then, if `ctx+0x75` is set, unlocks the buffer
(`vfunc +0x1C`, flags `0x8040`) and releases `ctx+8`. The context's final `Release` (`0x008306C0`, on the count reaching
0) calls slot 0 with flags 1, releases the texture `ctx+0x24` and the buffer `ctx+8` and frees the context, but
**does not unlock** (SR565-6). Slot 0 sets the vtable to the base `0x00AC17AC` (whose slots 1–5 are the purecall stub
`0x005D4A10`), frees `this` via `GlobalHeapFree` (`0x005E5620`) when bit 0 of `flags` is set, returns `this`.
`cGZDrawContextRenderToBuffer`'s constructor (`0x00830690`) clears `+0x24`, `+0x74`, `+0x75`, `+0xB0`.

**Destination record and context fields** written by Init (all from the locked buffer's interface):

| Context field | Source | Meaning to this class |
|---|---|---|
| `+0x78` | `vfunc +0x88` | pixel (0, 0) — record `+0` |
| `+0x7C` | `vfunc +0x8C` | row stride in bytes — record `+4` |
| `+0x80`, also `+0x0C` | `vfunc +0x24` | width — record `+8`, not read here |
| `+0x84`, also `+0x10` | `vfunc +0x28` | height — record `+0xC`, not read here |
| `+0x88` | `vfunc +0x3C`, first dword | colour type (5) — record `+0x10`, not read here |
| `+0x8C`, also `+0x90` | `vfunc +0x44` | bytes per pixel — record `+0x14`, not read here |

For a `cGZBuffer` (vtable `0x00AC1400`): `+0x24`/`+0x28` return the fields `+0x1C`/`+0x20`, `+0x3C` copies the pair
(`+0xC` type, `+0x10` bits) to the caller's record and returns that pointer, `+0x44` returns `bits >> 3`, `+0x88`
returns `+0x3C` (pixels), `+0x8C` returns `+0x40` (stride). Its lock (`0x00826AA0`) fails only when the pixel pointer is
null, otherwise ORs the flags into `+0x44`, bumps the word `+0x38` (and `+0x30` when bit 15 is set), returns 1; unlock
(`0x00826490`) clears the flags and decrements `+0x38`. The pointer and stride are cached once: the class assumes
2 bytes per pixel whatever `+0x8C` says.

**The only native caller** is `DrawPreclippedSpanList` (`0x008302E0`, thiscall `(spans, n)`, `RET 8`, slot 22 /
entry `0x00AC1828` of `0x00AC17D0`). It passes its own `spans`, `n` unchanged and `dst = ctx+0x78` always, and makes
exactly one renderer call (§6.1 chooses which):

| Path | Call site | Slot | Extra arguments |
|---|---|---|---|
| texture, tiled (`+0xA4`, `kind | 8` at `0x0083047F`) | `0x008304D1` | 5 | `src` = local copy of the texture record, `b` = local 32-byte record, `kind` |
| texture, stretch (`+0x40`, `kind | 4` at `0x0083050F`) | `0x0083053B` | 5 | same |
| texture, plain | `0x00830591` | 5 | same |
| no texture, gradient `+0x74`, alpha `+0x41` | `0x008305E7` | 4 | `grad = ctx+0x44` |
| no texture, gradient, no alpha | `0x008305FC` | 2 | `grad = ctx+0x44` |
| no texture, alpha | `0x00830627` | 3 | `colour = ctx+0xC4`, `argb = ctx+0xB4` |
| no texture, solid | `0x00830647` | 1 | `colour = ctx+0xC4` |

Texture paths lock the texture `ctx+0x24` (`vfunc +0x18`, `0x800`, `0x008302FB`; failure → return, nothing drawn, no
unlock) and unlock it (`vfunc +0x1C`, `0x800`) after the call or when its type selects nothing (`0x008305AA`). The
untextured paths take no lock: the destination stays locked from Init to Shutdown.

**Conformance examples (boundary).** (1) Slot 1, `spans = [{0x00010002, 0x00010005}]`, `n = 1`, `dst = {B, 12}`,
`colour = 0x1234F81F`: words at `B+16`, `B+18`, `B+20` become `0xF81F`, nothing else changes, 16 bytes popped.
(2) Slot 5, `b.+0x10 = 4` (A8888), `b.+0x1C = 0`, `kind = 0`, one one-pixel span, texel `0x00FF8040`: the word becomes
`0xFC08` (§7.2), 24 bytes popped.

### 1.2 Which buffers reach this class (re-span565-bridge4) [static; runtime open as SR565-5]

A 565 renderer exists only for a buffer whose `vfunc +0x3C` reports type 5 at `RenderToBuffer::Init`. Static sweep
of `.text` (`build-runs/scan5.py`: every `5` immediate next to a `0x10` immediate, every `PUSH 5` before the
colour-pair constructor `0x0044A490` or before a `CALL [reg+0xC]` (`cIGZBuffer::Init` slot)):

| Hit | What it is |
|---|---|
| `0x00826472` (type 5, bits 16) | the **only** type-5 producer: the cGZBuffer type resolver `0x00826390` (cGZBuffer vtable entry `0x00AC14B4`, screen-buffer entry `0x00ADB4CC`) |
| `0x0055FC62`, `0x008F07F7`, `0x00A21865`, `0x0099F86D`, `0x0055B54A` | message construction, a byte field, CRT formatting, an icon-drawing call with four immediates, a vehicle-manager call: no buffer type |
| colour-pair constructor | 0 calls with 5 |

The resolver runs inside every `cGZBuffer::Init` (`0x008269B0`, slot 3: `(width, height, type, bits)`, `RET 0x10`;
fails if already initialised or a dimension is 0; then slot `+0xA8` → `+0xAC` (`0x00826A50`) calls the resolver
and allocates `stride·H` with `stride = (W·(bits>>3) + 3) & ~3`). It changes only types 1 and 3: with the cached
`cIGZGraphicSystem` service (`RZ::GS`, `0x00913C1A`) answering `0xC47B747C`, display depth (low 6 bits of the first
dword its `+0x3C` fills) `≤ 16` → **7**/16, else 9/32; when that query fails, the service's `+0x24` fills the pair;
**only when no graphic-system service exists does a type-1/3 buffer become 5/16** (`0x00826472`). In a running
game the service exists and 16-bit modes give A4444 (`span-render-4444.md` §1.2), so **no shipped path that draws
spans into a type-5 RenderToBuffer target was found**. Not a proof: a buffer initialised before the graphic-system
service is registered would become type 5, and types read from files or computed at run time are outside the sweep.

**Original-vs-ours trace route (constructed fixture, not a game path).** Through GZCOM create `cGZBuffer` (CLSID
`0xC470D325`), call slot 3 `Init(W, H, 5, 16)` (type 5 is kept, stride `(2W + 3) & ~3`, e.g. W = 5 → 12 bytes, so row
padding is observable), get a context from `cIGZDrawSystem` slot 4 (`0x0082BAB0`, `(buffer, out)`; the RenderToBuffer
factory `0x0082B910` accepts any `0x20732180` object, allocates 0xD4 bytes, constructs, AddRefs into `*out`, does not
Init), call context slot 3 `Init(buffer)` (expect 1 and the vtable `0x00AC1844` at `[ctx+0xB0]`), set state through
the context's public setters, call slot 22 `DrawPreclippedSpanList(spans, n)`, read the pixel words back through
the buffer's `+0x88`/`+0x8C`, and finish with slot 4 `Shutdown` so the lock is undone. Run the same script against the
original and ours and compare every byte of the buffer including the padding. A report must label this
"constructed buffer"; a game-path claim needs SR565-5. The calls, slots and fields of that route are in §1.3.

### 1.3 Context public interface used by the route (re-span565-context4) [exact, static]

This is the framework **2D** `cGZDrawContextRenderToBuffer`, shared with
[span-render-4444.md §1.3](span-render-4444.md#13-the-buffer-draw-context-as-a-native-fixture-route-sp5-sr1555-7-re-span-context4-exact-static)
and span-render-1555.md §1.3. Applicability to 565 is proved by Init's type-5 jump-table entry
`0x008308BC` → `0x00830792`: it installs span vtable `0x00AC1844` (`0x008307A0`) in the same context's
`+0xB0` (`0x008307EE`). The context's vtable, setters and slot-22 consumer do not change. The separate 3D
`SC4DrawContext` in draw-context.md does not specify these setters. The contracts below were audited against
`re_query.py asm --bytes` and image dwords; the shared gradient precision proof is reused (§4.1).

All calls below are MSVC thiscall (`this` in ECX, stack arguments right to left, callee pops the stated bytes).
A byte result is tested in AL; upper EAX bits are not part of that result. A dash in Returns means no defined
API return value, even where instructions leave a value in EAX.

**Draw system slot 4** (`0x0082BAB0`, `(buffer, out)`, `RET 8` at `0x0082BADF` / `0x0082BAE8`): walks the circular
factory list whose head is `[this+0x20]` (node: `+0` next, `+8` factory), calling each factory's slot 3 (`+0x0C`)
with `(buffer, out)` and stops at the first that returns AL ≠ 0: returns **AL = 1**; empty list or every factory
refused: **AL = 0**. The RenderToBuffer factory (`0x0082B910`, `RET 8`) queries `buffer` for `0x20732180` (failure:
AL = 0, `*out` untouched); success: allocates 0xD4 bytes, runs the constructor `0x00830690`, writes the context into
`*out` without releasing what `*out` held, AddRefs it (count 0 → 1), releases its own query reference, AL = 1. A
failed allocation is outside this fixture contract. The constructor sets only the
vtable `0x00AC17D0`, `+4` (count), `+8` (target), `+0x24` (texture), `+0xB0` to 0 and the bytes `+0x41`, `+0x43`,
`+0x74`, `+0x75` to 0; every other field (`+0x28..+0x40`, `+0x42`, `+0x94..+0xAC`, `+0xB4..+0xD0`) holds whatever
the heap block held until a setter writes it.

**Context vtable `0x00AC17D0`** (23 entries, read from the image; entry 23 onward is the 1555 table `0x00AC182C`):

| Slot | Entry | Method | Arguments | Pops | Returns | Fields written |
|---|---|---|---|---|---|---|
| 0 | `0x0082FD60` | QueryInterface | `iid, out` | 8 | AL: 1 for iid 1 or `0x2AE78A07` (`*out = this`, AddRef), else 0 | — |
| 1 | `0x005F47F0` | AddRef | — | 0 | new count | `+4` |
| 2 | `0x008306C0` | Release | — | 0 | count | (final: §1.1) |
| 3 | `0x00830710` | Init | `buffer` | 4 (`0x00830877`) | AL 1 / 0 | target, `+0x78..+0x8C`, `+0xC`, `+0x10`, `+0x90`, `+0xB0`; `+0xA4 = 0`, `+0x75 = 1` |
| 4 | `0x0082FF20` | Shutdown | — | 0 | none defined | `+0xB0 = 0`, `+8 = 0` |
| 5 | `0x00A64430` | target bounds | `out` (16 bytes) | 4 | tail-jumps to target slot 11 (`+0x2C`): copies the buffer's `(0, 0, W, H)` into `out` | — |
| 6 | `0x0082FD90` | SetClipRect | `rect*` (4 dwords) | 4 | — | `+0x14..+0x20` (not read by slot 22) |
| 7 | `0x0082FF70` | SetColorTable | `index, count, argb*` | 0xC (`0x0082FFC0`) | — | per entry `k`: `+0xB4 + 4(index+k)` = ARGB, `+0xC4 + 4(index+k)` = native colour |
| 8 | `0x0082FEA0` | SetAlphaBlend | `int a` | 4 | — | `+0x41 = (a > 0)` signed |
| 9 | `0x0082FEB0` | SetBltModulate | `byte m` | 4 | — | `+0x42 = m` (low byte as passed) |
| 10 | `0x0082FEC0` | SetAntialiasing | `byte` | 4 | — | `+0x43` (not read by slot 22) |
| 11 | `0x00648F10` | (empty) | 1 dword | 4 | — | — |
| 12 | `0x0082FDB0` | SetGradientTransform | `p, x0, y0` | 0xC (`0x0082FE4D`) | — | `+0x44..+0x70`, `+0x74` (`span-render-1555.md` §4.1) |
| 13 | `0x0082FFD0` | SetBltSource | `tex, x, y, sx, sy, outRect*` (floats) | 0x18 (`0x00830278`, `0x00830296`) | AL (below) | `+0x24`, `+0x28..+0x40`, `+0xA8`, `+0xAC` |
| 14 | `0x0082FE60` | ClearBltTiling | — | 0 | — | `+0xA4 = 0` |
| 15 | `0x0082FE70` | SetBltTiling | `rect*` (x0, y0, x1, y1) | 4 | — | `+0x94..+0xA0`, `+0xA4 = 1` |
| 16, 20 | `0x00407990` | (empty) | — | 0 | — | — |
| 17 | `0x008302A0` | Suspend | — | 0 | — | if `+0x75`: target unlock `0x8040`, `+0x75 = 0` |
| 18 | `0x008302C0` | Resume | — | 0 | — | if not `+0x75`: target lock `0x8040` (result ignored), `+0x75 = 1` |
| 19 | `0x0082FED0` | LockTarget | `out` (24 bytes), 1 unused dword | 8 | AL = 1 | copies `+0x78..+0x8C` into `out` |
| 21 | `0x00831000` | Draw | `prim, verts*, count` (not used by this route) | 0xC | — | — |
| 22 | `0x008302E0` | DrawPreclippedSpanList | `spans, n` | 8 | — | — (§6.1, §6.2) |

**Init details used by §6.2.** On success Init stores the destination record `+0x78` pixels (buffer vfunc `+0x88`),
`+0x7C` stride (vfunc `+0x8C`), `+0x80` W (vfunc `+0x24`), `+0x84` H (vfunc `+0x28`), `+0x88` colour type
(first dword returned by vfunc `+0x3C`), `+0x8C` bytes per pixel (vfunc `+0x44`); also `+0xC` = W, `+0x10` = H,
`+0x90` = bytes per pixel; then `+0xA4 = 0` and
`+0x75 = 1` (`0x00830869`, `0x00830870`). It does not touch `+0x40`, `+0x41`, `+0x42`, `+0x74`, `+0xB4`, `+0xC4`.

**Shutdown** releases the renderer (slot 0, flags 1) if `+0xB0` is set and clears it; if a target is held it
unlocks it with `0x8040` **only when `+0x75` is set**, clears `+8` and tail-jumps to the target's Release, so EAX
is the target's new count on that path and left over otherwise: callers must not read it.

**SetColorTable** (`index, count, argb*`; `count = 0` writes nothing). For each of the `count` dwords `c`:
`+0xB4 + 4i = c` first, then `+0xC4 + 4i` = target slot 30 (`+0x78`, `0x00825DA0`) called with
`(c >> 16, c >> 8, c)` (it uses the low byte of each). For a type-5 target that converter is case `0x00825DE7`:
`(r & 0xF8) << 8 | (g & 0xFC) << 3 | b >> 3`, alpha dropped. `index` is not range-checked: entries 0–3 lie in the
object (`+0xB4..+0xC0`, `+0xC4..+0xD0`), index 4 writes its ARGB over native entry 0 and its native colour at
`+0xD4`, past the 0xD4-byte block. Here `i = index + k`. Slot 22 reads entry 0 only (`+0xB4`, `+0xC4`).
Store/call anchors: `0x0082FF86`–`0x0082FFAC`; RET `0x0082FFC0`; type-5 converter jump-table entry
`0x00825E64` → `0x00825DE7`, ending `0x00825E0E`. For `0x80FF8040` this gives `0xFC08`, while the
shared 1555 converter gives `0xFE08`: the setter is shared, its target-buffer packing is format-specific.

**Gradient and texture setters.** Slot 12 accepts 12 binary32 floats plus binary32 `(x0,y0)`; its exact
channel order, float stores, x87 precision and truncation are shared with span-render-1555.md §4.1 (also §4.1
below). The writes are `+0x44..+0x70`, followed by `+0x74 = 1` (`0x0082FDD0`–`0x0082FE45`); null input clears
only `+0x74` (`0x0082FE52`). Both exits pop 0xC (`0x0082FE4D`, `0x0082FE58`). Slot 13's texture state and
record construction are specified in §6.2. Colour and ARGB are supplied together by slot 7, not separate setters.

**Discriminating conformance vectors (specified, not executed here).** Use a constructed type-5 16×8 destination,
all spans in bounds, and explicitly set the path's state: slot 7 entry 0, slot 8 alpha, slot 9 modulation,
slot 12 gradient or null, slot 13 texture or null, slot 14 or 15 tiling. Do not depend on unset heap contents.
(1) SetColorTable(0, 1, [`0x80FF8040`]): `+0xB4 = 0x80FF8040`, `+0xC4 = 0xFC08`; then slot 22 with no texture,
alpha off, one span of 2 pixels writes `0xFC08` twice. (2) SetAlphaBlend(2) → `+0x41 = 1`; (−1) and
(`0x80000000`) → 0. (3) SetBltSource(tex 4×3, x = 2.5, y = 3.5, sx = sy = 1): `+0x28 = 2`, `+0x2C = 4` (half to
even), `+0x40 = 0`, `+0x30..+0x3C = 0x10000`, out = (2, 4, 6, 7), `+0xA8 = 4`, `+0xAC = 3`, AL = 1.
(4) SetBltSource(tex 4×3, x = 1, y = 0, sx = 2, sy = 0.5): `+0x30 = 0x20000`, `+0x34 = 0x8000`,
`+0x38 = 0x8000`, `+0x3C = 0x20000`, `+0x28 = 0x8000`, `+0x2C = 0`, `+0x40 = 1`, out = (1, 0, 9, 1),
`+0xA8 = 8`, `+0xAC = 1`. (5) SetBltSource(null, …) after (3): AL = 0, `+0x24 = 0`, `+0x28..` unchanged.
(6) SetBltModulate(`0x100`) stores byte 0; (`2`) stores byte 2 but selects modulation as a nonzero flag in slot 22.
(7) SetGradientTransform with twelve zero floats and origins 0 sets all four starts to `0x8000`, all steps to 0,
and `+0x74 = 1`; then null input clears the flag without rewriting the record. Both calls pop 0xC.
(8) Shutdown after Suspend: no unlock call (the `+0x75` gate), target still released. Shutdown takes no stack
arguments; it is void even when its tail call leaves the buffer's Release result in EAX.

## 2. RenderSolid (`0x0083DE50`, slot 1) [exact, verified]

Arguments: spans, n, destination, colour (dword); `ret 0x10`. Per span it calls the 16-bit fill helper
`0x0090FFC8(dest, value, count)`, which stores the **low 16 bits of the colour verbatim** into `count` consecutive
pixels (a leading word if the address is not 4-aligned, dword pairs, a trailing word; observably `count` copies).
The caller passes an already packed 565 value; no conversion happens here.

## 3. RenderAlpha (`0x0083DFB0`, slot 3) [exact, verified]

Arguments: spans, n, destination, `src` (dword; its low 16 bits are a packed 565 colour, bits 16–31 ignored),
`argb` (dword; only bits 24–31, `a8`, are read); `ret 0x14`.

1. **Weight** `w = (a8 × 0x818180) >> 26` (32-bit, unsigned), `w ∈ 0…32`. First `a8` giving each `w`:
   1:8 2:16 3:24 4:32 5:40 6:48 7:56 8:64 9:72 10:80 11:87 12:95 13:103 14:111 15:119 16:127 17:135 18:143 19:151
   20:159 21:167 22:174 23:182 24:190 25:198 26:206 27:214 28:222 29:230 30:238 31:246 32:254. So `a8 ≤ 7` leaves
   the destination unchanged and `a8 ≥ 254` replaces it.
2. **Blend `B(S, D, w)`** (the same rule as helper `0x00845450`, used again in §5 and §7):
   - G term: `((S & 0x7E0) − (D & 0x7E0)) × w + ((D & 0x7E0) + 0x10) × 32`, masked `& 0xFC00`;
   - R+B term: `((S & 0xF81F) − (D & 0xF81F)) × w + (D & 0xF81F) × 32 + 0x8010`, masked `& 0x1F03E0`;
   - result = `(G term + R+B term) >> 5`, low 16 bits; all arithmetic 32-bit wrapping.
   For `w ≤ 32` this equals, per channel independently, `C = floor((w × S_c + (32 − w) × D_c + 16) / 32)` with
   5-bit R/B and 6-bit G (the sums stay below 1024 resp. 2048, so no carries cross channels). RenderAlpha masks
   before adding; §5 instead ORs `0x07E0` / `0xF81F` into the two shifted terms and ANDs them: the two forms give
   the same 16 bits for every input.

Worked example: D = `0x0000`, src = `0xFFFF`, `a8 = 0x80` → `w = 16`; R = B = `floor((16·31 + 16)/32) = 16`,
G = `floor((16·63 + 16)/32) = 32` → `0x8410`.

## 4. RenderGradient (`0x0083DEA0`, slot 2) [exact, verified]

Arguments: spans, n, destination, `g` (pointer to the gradient record, §4.1); `ret 0x10`.

### 4.1 Gradient record (0x30 bytes) and its producer [exact]

Twelve dwords at `ctx+0x44`: `+0x00..+0x0C` start values `c0..c3`, `+0x10..+0x1C` per-x steps `dx0..dx3`,
`+0x20..+0x2C` per-y steps `dy0..dy3`. Channel `i` at absolute pixel (x, y) is `V_i = c_i + dx_i·x + dy_i·y`
(32-bit wrapping). `c0`/`V0` is **B**, `V1` **G**, `V2` **R**, `V3` the alpha weight (§5 only).

Producer: `cGZDrawContextRenderToBuffer::SetGradientTransform` (`0x0082FDB0`; `this`, `p` = 12 floats, `x0`, `y0`
floats; `ret 0xC`). `p` null → clears the gradient flag `+0x74` and returns. Otherwise for `i = 0…3`:
`dx_i = R(p[4+i] × 16711680.0)`, `dy_i = R(p[8+i] × 16711680.0)` (double constant at `0x00AC17C8` = 255 × 65536;
`R` = product rounded to a 32-bit float, then `fistp` in the current FPU rounding mode, nearest-even by default),
`s_i = R(p[i] × 16711680.0) + 0x8000`, and `c_i = trunc(s_i − (dx_i·x0 + dy_i·y0))` on the x87 stack (`fild` of the
integers, `x0`/`y0` as float, `faddp` then `fsubp` = `s − sum`, truncation by `_ftol` `0x009EEF04`). Then
`+0x74 = 1`. So a float 1.0 becomes 255 in bits 16–23 (8.16 fixed point), the start is pre-rounded by one half, and
`(x0, y0)` is the pixel where the colour equals `p[0..3]`. No direct caller of `0x0082FDB0` (or of its base
forwarder) exists in the export: whether shipped code ever draws a gradient is open (SR565-1).

### 4.2 Per-pixel rule

At the span start the code evaluates `V0, V1, V2` exactly at (start.x, y), then keeps three accumulators:

| Channel | Accumulator start | Step per pixel | Output bits |
|---|---|---|---|
| B | `V0 >> 3` (**logical**) | `dx0 >> 3` (**arithmetic**) | accumulator bits 16–20 → B |
| G | `V1 << 3` | `dx1 << 3` | accumulator bits 21–26 → G |
| R | `V2 << 8` | `dx2 << 8` | accumulator bits 27–31 → R |

Pixel = `R << 11 | G << 5 | B`. Equivalently: **R = bits 19–23 and G = bits 18–23 of the exact value
`V + k·dx`** (the top 5 / 6 bits of the 8-bit channel, truncated; the shifts are exact modulo 2³²), while **B is the
top 5 bits of the 8-bit channel computed from a start shifted right by 3 and a step whose low 3 bits are dropped
(rounded toward −∞)**. B therefore drifts from the exact value along a span when `dx0` is not a multiple of 8.
Worked example: `V0 = 0x0007FFF8`, `dx0 = 7`: the exact value at the third pixel is `0x00080006` (B = 1), but the
shipped accumulator stays `0xFFFF` (B = 0). With the producer's values (1.0 → `0x00FF8000`) a constant white
gives `0xFFFF`.

## 5. RenderAlphaGradient (`0x0083E070`, slot 4) [exact, verified]

Arguments as §4; `ret 0x10`. The colour `S` is formed exactly as §4.2 (same three accumulators). The weight uses a
fourth accumulator `A = V3 × 0x84` stepped by `dx3 × 0x84` (both 32-bit wrapping, so `A = (V3 + k·dx3) × 0x84`
exactly), and `w = A >> 26` (unsigned, `0…63`). Each pixel becomes `B(S, D, w)` of §3. For an in-range alpha
(`V3 ≤ 0x00FFFFFF`) `w ≤ 32` and the per-channel lerp applies; out-of-range values give `w` up to 63 and the
packed formula of §3 (with its inter-channel borrows) is the rule. Example: `V3 = 0x00FF8000` → `w = 32`.

## 6. RenderBlt (`0x0083E210`, slot 5) [exact, verified]

Arguments (`ret 0x18`): spans, n, `dst` (pointer, 24 bytes), `src` (pointer, 24 bytes), `b` (pointer, 32 bytes),
`kind`. It copies the three records onto the stack and calls `row[kind](spans, n, *dst, *src, *b)` (cdecl, 0x58
bytes of arguments) with `row = rows[byte b+0x1C + 2 × dword b+0x10]`; `rows` is the 14-pointer array at
`0x00AC1E28` (row `i` at `0x00AC1DF8 − 0x30·i`, 12 entries each, block `0x00AC1B88`–`0x00AC1E27`). Neither index is
range-checked.

`src` record: `+0` pointer to texel (0, 0), `+4` stride in bytes, `+8` width W, `+0xC` height H. `b` record:
`+0` u0, `+4` v0, `+8` du, `+0xC` dv, `+0x10` source-format index `f`, `+0x14` ARGB draw colour, `+0x18` colour
key, `+0x1C` key flag (byte).

### 6.1 How the caller fills them (`DrawPreclippedSpanList`, `0x008302E0`) [exact]

No texture (`ctx+0x24` null): gradient flag `+0x74` → RenderAlphaGradient if alpha flag `+0x41` else
RenderGradient (record `ctx+0x44`); otherwise RenderAlpha(`src = ctx+0xC4`, `argb = ctx+0xB4`) if `+0x41` else
RenderSolid(`ctx+0xC4`). The destination record is always `ctx+0x78`. With a texture it locks it (vfunc `+0x18`,
flag `0x800`) and:

- **Format index** from the texture colour type (jump table `0x00830654`): 7 (A4444) → 0, 6 (A1R5G5B5) → 1,
  4 (R5G5B5) → 2, 5 (R5G6B5) → 3, 9 (A8R8G8B8) → 4, 8 (R8G8B8) → 5, 2 (8-bit) → 6; 3 or anything outside 2…9 →
  nothing drawn (unlock, return).
- **Key** (texture vfunc `+0x68` returns the flag, key into `b+0x18`), masked per format (table `0x00830674`):
  0 `& 0x0FFF`, 1 and 2 `& 0x7FFF`, 3 `& 0xFFFF`, 4 and 5 `& 0x00FFFFFF`; for 6 the flag is forced to 0, so row 12
  is the only 8-bit row reached and row 13 is unreachable.
- `b+0x14 = ctx+0xB4`. `kind = alpha + 2·modulate` (nonzero tests of `+0x41`, `+0x42`), then `| 8` if tiling (`+0xA4`), else `| 4`
  if stretching (`+0x40`); tiling wins.
- **Plain**: source pointer biased by `−(ox·bpp + oy·stride)` (`ox = ctx+0x28`, `oy = ctx+0x2C`), so destination
  (x, y) reads texel (x − ox, y − oy).
- **Stretch**: pointer unbiased; `du = ctx+0x38`, `dv = ctx+0x3C` (16.16), `u0 = (du SAR 1) − ox`,
  `v0 = (dv SAR 1) − oy` (`ox`, `oy` already 16.16 here): texel = ((x + ½)·du − ox, (y + ½)·dv − oy) >> 16.
- **Tiled**: the sub-rectangle `ctx+0x94..+0xA0` (x0, y0, x1, y1) of the texture: pointer += `x0·bpp + y0·stride`,
  `W = x1 − x0`, `H = y1 − y0`, `u0 = (K_W − ox) mod W` with `K_W = 0x7FFFFFFF − (0x7FFFFFFF mod W)` (unsigned
  `div`), `v0` likewise with H and `oy`: `u0 ≡ −ox (mod W)` for `ox ≤ K_W`. `W = 0` or `H = 0` faults (divide).

### 6.2 Texture state provenance and the public-setter route (SR565-8) [exact, static]

There is **no persistent 24-byte source surface record in the context**. `+0x24` holds a referenced `cIGZBuffer`
interface, not the pixels or a surface-record pointer. Slot 13 SetBltSource (`0x0082FFD0`) releases the old
reference, clears it, and queries the new object for `0x20732180` into `+0x24` (`0x0082FFE4`–`0x00830008`). Null
source releases/clears the held reference and returns AL = 0; a source refusing the interface returns AL = 1.
Neither case writes the placement/scales or caller's output rectangle. The success and null exits pop 0x18 at
`0x00830278` / `0x00830296`. Thus AL alone does not prove that a texture was installed.

On success the setter reads source bounds `(L,T,R,B)` through buffer slot 11 (`0x00830017`); `x,y,sx,sy` are
binary32 arguments describing destination placement and destination size per source texel. The output pointer
must name a writable 16-byte **edge rectangle**, not a crop request. These rules are the same shared 2D-context
rules documented in span-render-4444.md §1.3 (including its source-placement proof), not a 565 variation:

| Context field(s) | Producer and meaning |
|---|---|
| `+0x24` | slot 13: held source interface |
| `+0x30`, `+0x34` | slot 13: `R(sx·65536)`, `R(sy·65536)` (forward 16.16 scales) |
| `+0x38`, `+0x3C` | slot 13: `du=R(65536/sx)`, `dv=R(65536/sy)` (source steps) |
| `+0x28`, `+0x2C`, byte `+0x40` | slot 13: if both stored forward scales are `0x10000`, integer `R(x),R(y)` and mode 0; otherwise 16.16 `R(du·x),R(dv·y)` and mode 1 |
| `+0x94..+0xA0`, byte `+0xA4` | slot 15 SetBltTiling copies source-texel edges `(left,top,right,bottom)` and sets flag 1; slot 14 clears only the flag; Init also clears it |
| `+0xA8`, `+0xAC` | slot 13: output right-minus-left and bottom-minus-top, wrapping dword subtraction; these are not the source surface's width/height |

`R` retains the caller's x87 precision for the arithmetic, stores/reloads binary32, then converts to signed dword
using the current rounding mode (`FISTP`). Plain placement converts the already-binary32 argument directly.
The shared game's 24-bit, nearest-even rule applies (span-render-1555.md §4.1); the setter does not change the
control word. `65536.0` is a **double** constant at `0x00A8CEB0`, not an instruction to compute the whole expression
in double. Anchors: `0x0083001A`–`0x00830082` (scales), `0x00830085`–`0x008300B3` (quantized mode/plain placement),
`0x008300F6`–`0x0083012E` (scaled placement).

In plain mode the output is `(L+ox,T+oy,R+ox,B+oy)` (`0x008300B7`–`0x008300F1`, `0x00830259`). In stretch mode
its edge inputs are `x,y,x+(R-L)·sx,y+(B-T)·sy`, with the caller's x87 precision at each operation. The exact
edge-conversion instruction proof and precision limits are shared with span-render-4444.md §1.3.1; for the finite,
small coordinates in §1.3's vectors, each edge is `ceil(t−0.5)` in nearest-even mode. Do not extend that shorthand
to arbitrary floats. Width/height stores are `0x00830256`–`0x0083026A`. SetBltSource does not change tiling.
SetBltTiling writes its own fields at `0x0082FE74`–`0x0082FE94`, pops 4 at `0x0082FE97`; ClearBltTiling returns
with no pop at `0x0082FE67`. Both are void.

For each slot-22 draw with a texture, after Lock(`0x800`) succeeds, the caller constructs `src` in **its stack
frame** using these buffer vfuncs (`0x00830306`–`0x00830357`):

| RenderBlt `src` member | Source at draw time |
|---|---|
| `+0` pixels | source buffer slot 34 (`+0x88`) |
| `+4` stride | source buffer slot 35 (`+0x8C`) |
| `+8` width, `+0xC` height | source buffer slots 9/10 (`+0x24`, `+0x28`) |
| `+0x10` colour type | first dword returned by slot 15 (`+0x3C`) |
| `+0x14` bytes per texel | source buffer slot 17 (`+0x44`) |

The local `b` record then receives the placement/steps, format/key and ARGB of §6.1. Modulation is tested as
`+0x42 != 0`, not used as a numeric multiplier when selecting `kind` (`0x0083040D`–`0x00830431`). Plain mode
biases the local pixel pointer; stretch leaves it unchanged; tiling biases it and replaces the local width/height
with the tile's edge differences (§6.1). The pointers to these local records and `ctx+0x78` go to the renderer's
slot 5 (calls `0x008304D1`, `0x0083053B`, `0x00830591`); RenderBlt then makes the by-value copies described in §6.
None of those adjustments writes a source surface back into the context or changes the source buffer's metadata.

**Additional conformance vectors (specified, not executed here).** Use the public setters and slot 22 on the
type-5 destination of §1.3; use an unkeyed type-5 4×3 source with distinct texel values, alpha/modulation off.
With placement `(2,1)`, scales `(1,1)`, and tiling cleared, one pixel at destination `(2,1)` copies source `(0,0)`.
SetBltTiling `(1,1,4,3)` instead: local source dimensions become 3×2, the pixel pointer advances by
`2 + sourceStride`, and destination `(2,1),(3,1),(5,1)` copy original texels `(1,1),(2,1),(1,1)`. The input rect
stays unchanged; ClearBltTiling restores the plain mapping. With §1.3 vector (4)'s placement/scales, destination
`(1,0)` copies source `(0,1)` (the half-step centre distinguishes it from corner sampling). Check all six `src`
members at renderer entry against the source buffer plus the stated mode adjustments, not destination fields
or `+0xA8/+0xAC`.

Null-source and refusing-interface cases must leave a sentinel output rectangle unchanged, while returning
0 and 1 respectively and leaving `+0x24` null; neither draws a texture on the next slot-22 call. These are
constructed native-context vectors. This static proof does not establish a shipped 565 drawing path (SR565-5).

## 7. The 565 dispatch table (row = `2·f + key`, column = `kind`) [verified, all 168 slots]

Columns: 0 `Blt`, 1 `BltConstantAlpha`, 2 `BltModulate`, 3 `BltModulateAlpha`, then the same four `Stretch`
(4–7) and `Tiled` (8–11) (Mac-build method names; the column ↔ flag mapping of §6.1 is exact). `*` = body outside
this owner range (shared by identical-code folding).

| Row | Source, key | Columns 0–11 (`0x008…`) |
|---|---|---|
| 0 | A4444, off | 46CD0 3EF00 46D80 3EFD0 46E70 3F140 46F30 3F280 47040 3F420 47110 3F570 |
| 1 | A4444, on | 47240 3F710 47310 3F800 47420 3F990 474F0 3FAE0 47610 3FC90 476F0 3FE10 |
| 2 | A1555, off | 40980 3E2B0 40AF0 3FFD0 40D50 3E360 40F50 40140 41200 3E480 41440 402E0 |
| 3 | A1555, on | 416F0 3E5B0 41890 40480 41B20 3E670 41D40 40610 42010 3E7A0 42290 407C0 |
| 4 | X555, off | 40980 40A20 40AF0 40BE0 40D50 40E10 40F50 41070 41200 412E0 41440 41560 |
| 5 | X555, on | 416F0 417B0 41890 419A0 41B20 41BF0 41D40 41E70 42010 42110 42290 423C0 |
| 6 | R565, off | 48160* 42570 42600 42700 33730* 42880 42990 42AB0 42C50 42CF0 42E10 42F40 |
| 7 | R565, on | 430E0 43160 43200 43320 434B0 43550 43670 437A0 43950 43A00 43B40 43C90 |
| 8 | A8888, off | 44680 3E8F0 447E0 43E50 44A00 3E990 44C00 43F80 44E50 3EAB0 45060 440E0 |
| 9 | A8888, on | 452B0 3EBE0 454C0 44240 456E0 3EC90 45910 44390 45B70 3EDC0 45DD0 44500 |
| 10 | X888, off | 44680 44720 447E0 448B0 44A00 44AC0 44C00 44CE0 44E50 44F20 45060 45140 |
| 11 | X888, on | 452B0 45370 454C0 45590 456E0 457C0 45910 45A00 45B70 45C70 45DD0 45EE0 |
| 12 | 8-bit, off | 47830 46060 47880 460D0 47830 46210 47880 462C0 47920 46430 47990 46500 |
| 13 | 8-bit, on (unreachable) | 47A50 46670 47B00 466F0 47AA0 46840 47B00 46900 47BB0 46A70 47C20 46B50 |

141 distinct bodies (139 here, plus `0x00848160`, the 16-bit ascending word copy shared with other tables, and
`0x00833730` in the 1555 range). The A-format and X-format rows share every non-alpha column (0, 2, 4, 6, 8, 10):
with no destination alpha, source alpha matters only in the alpha columns. The 8-bit Blt and Modulate bodies ignore
the texel, so their plain and stretch slots are one body each. Two foreign tables (`0x00AC1AD4`, `0x00AC20F0`)
point at the 565 body `0x00842C50`.

### 7.1 Body calling convention

cdecl `(spans, n, dst, src, b)` with the three records by value (0x58 bytes). Spans are walked as §1.

### 7.2 Source texel → 565 (column 0 `Blt`, and the colour used by `ConstantAlpha`)

| f | Source | Conversion |
|---|---|---|
| 0 | A4444 | each 4-bit channel widened by bit replication: R5 = `r<<1 | r>>3`, G6 = `g<<2 | g>>2`, B5 = `b<<1 | b>>3`; A ignored |
| 1, 2 | A1555, X555 | R and B copied, G6 = `g<<1 | g>>4`; bit 15 ignored |
| 3 | R565 | copied |
| 4, 5 | A8888, X888 | R5 = `R8>>3`, G6 = `G8>>2`, B5 = `B8>>3` (truncation); A ignored |
| 6 | 8-bit | `0xFFFF` (white) whatever the texel |

Examples: A4444 `0x0F8C` → `0xFC59`; X555 `0x0210` → `0x0430`; X888 `0x00FF8040` → `0xFC08`.

### 7.3 ConstantAlpha, Modulate, ModulateAlpha

Let `T` = the §7.2 value and `(ca, cr, cg, cb)` the bytes of `b+0x14`.

- **ConstantAlpha** (column 1): `B(T, D, ca >> 3)` (§3). The weight is `0…31`: even `ca = 255` leaves 1/32 of the
  destination (white over black → R = B = 30, G = 61).
- **Modulate** (column 2): expand the texel to 8 bits per channel — A4444: `x·17`; 1555/555: 5-bit `v<<3 | v>>2`;
  565: R/B as 5-bit, G `v<<2 | v>>4`; 8888/888: the bytes; 8-bit: A = texel, R = G = B = 255; A = 255 for X555,
  R565 and X888 — then per channel `m = (c·cc + 128) >> 8` (`cc` the matching colour byte; never above 255), and pack
  `R5 = mR>>3, G6 = mG>>2, B5 = mB>>3`. Example: R565 `0xFFFF` with colour `0x80808080` → `0x8410`.
- **ModulateAlpha** (column 3): the Modulate value `P` and modulated alpha `mA`, written as `B(P, D, mA >> 3)`
  (weight `0…31`; for the 8-bit source `mA = (texel·ca + 128) >> 8`).
- **Key** (odd rows): when the flag is set the pixel is left untouched if `(texel & mask_f) == key`, masks as §6.1;
  the 8-bit keyed row compares no texel bits, so it draws every pixel when `key ≠ 0` and none when `key = 0`.

### 7.4 Addressing (columns 4–7 stretch, 8–11 tiled)

- **Stretch**: texel (`(u0 + du·x) >> 16`, `(v0 + dv·y) >> 16`) with x, y the absolute destination pixel, 32-bit
  wrapping sums and **logical** shifts (every stretch body uses `shr`, 156 shifts checked). No bounds check.
- **Tiled**: at each span start `sx = ((x + u0) mod 2³²) mod W`, `sy = ((y + v0) mod 2³²) mod H` (unsigned `div`);
  then `sx` steps by 1 and wraps to 0 at W.

## 8. Proofs and how to rerun them

Scripts in `docs/specs/proofs/` (need `unicorn` and the GOG 1.1.641 `SimCity4.exe`; set `SC4EXE` for another
path). They load the PE image into the emulator and run the shipped bytes on random inputs against the rules above:

| Script | What it checks | Result (2026-10-21) |
|---|---|---|
| `span565_methods.py 500` | §2–§5 on random spans/colours and full-range gradient records; bytes popped; RenderBlt reaches `rows[key + 2f][kind]` | 5/5 PASS 500 |
| `span565_blt.py 300` | all 168 slots, random sources, colours, keys, strides, stretch steps, tiled origins (incl. 32-bit u0/v0) | 168/168 |
| `span565_blt.py via 100` | the same through RenderBlt with records by pointer | 168/168 |
| `span565_sweep.py 4` | every 16-bit texel (256 8-bit, 65 536 random 32-bit with alpha 0/255) per slot, 4 colours | 168/168 |
| `span565_bridge.py` | §1.1: the six vtable entries read from the image, every slot called through its entry with an unmapped `this`, bytes popped, EBX/EBP/ESI/EDI preserved, both boundary examples, slot 0 with flags 0 | PASS |
| `span565_alias.py 200` | source aliased onto the destination (§1 overlap rule); a snapshot model fails 148 slots | 168/168 |

## 9. Open points

- **SR565-1** (§4.1): no direct caller of `SetGradientTransform` exists, so whether any shipped path draws a
  gradient into a type-5 buffer (where the B drift of §4.2 would show) is unknown.
- **SR565-5** (§1.2): runtime check that no shipped path creates a 565 renderer (static sweep found only the
  no-graphic-system fallback `0x00826472`).
- **SR565-6** (§1.1): whether the destination lock left behind by Init's type-3 exit and by Release without
  Shutdown must be reproduced.
- Behaviour with records the caller never produces (stretch reads past the texture, `W = 0` tiled, row 13) is
  specified only as far as the tables above say; no clipping exists to fall back on.
