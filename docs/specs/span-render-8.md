# Span renderer, 8-bit (`nGZDrawSoftRender::cSpanRender8`)

Owner range `0x0085E640`–`0x00864B58` (130 functions): the five methods of vtable `0x00AC188C` (slot 0 is the
shared base destructor `0x0082FF00`, not owned here) and 125 blitter bodies `0x0085E7F0`–`0x00864A90` reached only
through the RenderBlt table. This is the span back end that `cGZDrawContextRenderToBuffer::Init` (`0x00830710`)
creates into `+0xB0` for buffer colour type **2 (8 bits per pixel)** only (Init's jump table `0x008308B0`, entry for
type 2 → `0x008307D4`, which stores vtable `0x00AC188C`). Everything below is derived from the 8-bit bytes and
re-checked by the proofs of §8; the 565/1555/4444/8888 siblings are cited only for the shared caller.

Status legend: **[exact]** = every instruction read and restated; **[verified]** = checked against the shipped
bytes by execution under an x86 emulator (§8). Every function of the owner range is [exact] or [verified].

**What a pixel is.** One byte per pixel (`u8`). This class never interprets the byte as a palette index or as a
colour: it stores bytes it is given (§2, §3), the top byte of a fixed-point ramp (§4), or a value derived **only from
the source texel's alpha** and the draw colour's alpha byte (§7). No RGB channel of any source or of the draw colour
is ever read by a blit body, and no blit body reads the destination (a keyed pixel is simply skipped). So the
blit and gradient paths behave as if the buffer were an **alpha (coverage) plane**; the solid paths store the
buffer-mapped colour, which is always 0 for a type-2 buffer (§10.4). How a context reaches this class: §10.

## 1. Common inputs [exact]

- **Span list.** `n` records of 8 bytes: `+0` start, `+4` end, each a packed point (x low 16 bits, y high 16 bits,
  unsigned). Pixel count = `end − start` as a 32-bit difference: valid only on one row with `end.x ≥ start.x`;
  pixels `start.x … end.x − 1` are written left to right. A zero-length span writes nothing (every method and body
  tests it or uses a zero repeat count).
- **Count `n`.** Every method and body loops `do … while (--n)`: `n = 0` is **not** a no-op (wraps, walks 2³²
  records). *[compat: callers never pass 0; the original crashes]*
- **Destination record** (24 bytes; methods take a pointer, bodies a copy): `+0` pointer to pixel (0, 0), `+4` row
  stride in bytes (32-bit multiply, a negative stride works), `+8` width, `+0xC` height, `+0x10` colour type, `+0x14`
  bytes per pixel. Only `+0` and `+4` are read: **no clipping, no bounds check**. Pixel address =
  `base + y × stride + x`.
- **Order and overlap.** Spans in list order, each fully before the next, pixels in ascending x; each pixel's
  texel is read and its byte written before the next pixel is touched, so a pixel covered twice is processed twice and
  a source aliased onto the destination behaves exactly as an ascending byte-by-byte copy (proved for all 168 slots,
  `span8_alias.py`). The fill bodies (§7.3) store with `rep stosd` / `rep stosb`; they read nothing, so this is
  observably `count` stores of `0xFF`.

## 2. RenderSolid (`0x0085E640`, slot 1) [exact, verified]

Arguments: spans, n, destination, colour (dword); `ret 0x10`. Every span pixel receives the **low byte of the
colour** verbatim (bits 8–31 ignored); stored as dwords of four copies, then the remaining 0–3 bytes. No conversion.
Example: colour `0x12345678` → every pixel `0x78`.

## 3. RenderAlpha (`0x0085E720`, slot 3) [exact, verified]

Arguments: spans, n, destination, `src` (dword), `argb` (dword); `ret 0x14`. It **ignores `argb` entirely** and
makes the virtual call `this->vtable[1](spans, n, destination, src)`, i.e. RenderSolid (§2) for this vtable: no
blending happens. Example: `src = 0x000000AB`, `argb = 0x10FFFFFF` → every pixel `0xAB`.

## 4. RenderGradient (`0x0085E6A0`, slot 2) [exact, verified]

Arguments: spans, n, destination, `g` (pointer to the 0x30-byte gradient record at `ctx+0x44`: `+0x00..+0x0C`
start values `c0..c3`, `+0x10..+0x1C` per-x steps, `+0x20..+0x2C` per-y steps; producer `SetGradientTransform`
`0x0082FDB0`, see `span-render-565.md` §4.1); `ret 0x10`. **Only the fourth channel is read**: `c3` (`+0x0C`),
`dx3` (`+0x1C`), `dy3` (`+0x2C`) — the channel the 565 renderer uses as alpha weight. At the span start the code
forms `V = c3 + dy3·y + dx3·start.x` (32-bit wrapping), stores **bits 16–23 of `V`** (`(V >> 16) & 0xFF`), then adds
`dx3` per pixel; this equals `(c3 + dx3·x + dy3·y) >> 16 & 0xFF` exactly for every pixel. Channels 0–2 are ignored.
Side effect: the method overwrites its own fourth argument slot on the stack with `dx3` (caller-owned scratch, no
observable effect for a C caller).

Example: `c3 = 0x00FF8000` (the producer's value for alpha 1.0), `dx3 = −0x00010000`, `dy3 = 0`: pixels from x = 0
are `0xFF, 0xFE, 0xFD, …`.

## 5. RenderAlphaGradient (`0x0085E740`, slot 4) [exact, verified]

Arguments as §4. The body is a tail jump to `this->vtable[2]` with the same stack, i.e. RenderGradient (§4) for
this vtable; `ret 0x10` is RenderGradient's. No blending.

## 6. RenderBlt (`0x0085E750`, slot 5) [exact, verified]

Arguments (`ret 0x18`): spans, n, `dst` (pointer, 24 bytes), `src` (pointer, 24 bytes), `b` (pointer, 32 bytes),
`kind`. Copies the three records onto the stack and calls `row[kind](spans, n, *dst, *src, *b)` (cdecl, 0x58 bytes
of arguments, caller pops) with `row = rows[byte b+0x1C + 2 × dword b+0x10]`; `rows` is the 14-pointer array at
`0x00AC26B0` (row `i` at `0x00AC2680 − 0x30·i`, 12 entries each, block `0x00AC2410`–`0x00AC26AF`). Neither index
is range-checked.

`src` record: `+0` pointer to texel (0, 0), `+4` stride in bytes, `+8` width W, `+0xC` height H (`+0x10`, `+0x14`
not read). `b` record: `+0` u0, `+4` v0, `+8` du, `+0xC` dv, `+0x10` source-format index `f`, `+0x14` ARGB draw
colour (only byte 3, `ca`, is ever used), `+0x18` colour key (dword), `+0x1C` key flag (byte).

The caller (`DrawPreclippedSpanList`, `0x008302E0`) is shared with the other span renderers and is specified in
`span-render-565.md` §6.1: format index from the texture colour type (A4444 → 0, A1R5G5B5 → 1, R5G5B5 → 2,
R5G6B5 → 3, A8R8G8B8 → 4, R8G8B8 → 5, 8-bit → 6), key masked per format, the key flag forced to 0 for format 6 (so
row 13 is unreachable), `b+0x14 = ctx+0xB4`, `kind = alpha + 2·modulate | 8 tiled | 4 stretch` (tiling wins), and
the plain/stretch/tiled pointer and origin set-up. The solid colour `ctx+0xC4` of §2/§3 is written by
`SetColorTable` (`0x0082FF70`) as the result of the target buffer's vfunc `+0x78(r, g, b)` on the ARGB colour (the
raw ARGB goes to `ctx+0xB4`); that mapping belongs to the buffer class, not to this owner (SR8-1).

## 7. The 8-bit dispatch table (row = `2·f + key`, column = `kind`) [verified, all 168 slots]

Columns: 0 `Blt`, 1 `BltConstantAlpha`, 2 `BltModulate`, 3 `BltModulateAlpha`, then the same four `Stretch` (4–7)
and `Tiled` (8–11). `F` = fill body `0x0085FCB0` (plain and stretch), `T` = tiled fill body `0x00861BB0` (§7.3).

| Row | Source, key | Columns 0–11 (`0x008…`) |
|---|---|---|
| 0 | A4444, off | 5E7F0 F 5E870 5E950 5EA30 F 5EAD0 5EBD0 5ECD0 T 5ED80 5EEA0 |
| 1 | A4444, on | 5EFC0 5F050 5F0D0 5F1D0 5F2D0 5F380 5F410 5F520 5F630 5F6E0 5F780 5F8B0 |
| 2 | A1555, off | 5F9E0 F 5FA60 5FB40 5FC20 F 5FD00 5FE20 5FF30 T 5FFC0 600D0 |
| 3 | A1555, on | 601E0 60260 602E0 603E0 604E0 60580 60610 60730 60850 61510 60900 60A30 |
| 4 | X555, off | F F 60B50 60C20 F F 60CF0 60E00 T T 60F00 61010 |
| 5 | X555, on | 60260 60260 61110 61200 60580 60580 612F0 61400 61510 61510 615B0 616E0 |
| 6 | R565, off | F F 617F0 618D0 F F 619B0 61AB0 T T 61C30 61D50 |
| 7 | R565, on | 61E70 61E70 61EE0 61FE0 620E0 620E0 62170 62280 62390 62390 62420 62560 |
| 8 | A8888, off | 62690 F 62700 627B0 62850 F 628E0 629A0 62A60 T 62AF0 62BD0 |
| 9 | A8888, on | 62C90 637B0 62D10 62DC0 62E70 63990 62F10 62FE0 630B0 63BC0 63150 63230 |
| 10 | X888, off | F F 63310 633C0 F F 63470 63540 T T 63610 636E0 |
| 11 | X888, on | 637B0 637B0 63830 638E0 63990 63990 63A20 63AF0 63BC0 63BC0 63C50 63D40 |
| 12 | 8-bit, off | 63E10 F 63E70 63F20 63FD0 F 64050 64120 641F0 T 64270 64350 |
| 13 | 8-bit, on (unreachable) | 64420 64480 644D0 64580 64630 646C0 64710 647E0 648B0 64940 649B0 64A90 |

125 distinct bodies, all in the owner range (no identical-code folding with other owners). Bodies shared across
slots compute the same thing because the slots agree under the rules below (e.g. a keyed X555 `Blt` and a keyed
A1555 `ConstantAlpha` both store `0xFF` on every non-keyed pixel, body `0x00860260`).

### 7.1 Body calling convention

cdecl `(spans, n, dst, src, b)` with the three records by value (0x58 bytes); spans walked as §1.

### 7.2 Source texel → alpha `A` (the only texel property any body uses)

| f | Source | `A` |
|---|---|---|
| 0 | A4444 | top nibble × 17 (`a<<4 | a`): `0x8F00` → `0x88` |
| 1 | A1555 | bit 15 set → `0xFF`, else `0x00` (`0x7FFF` → `0x00`, `0x8000` → `0xFF`) |
| 2, 3 | X555, R565 | `0xFF` |
| 4 | A8888 | byte 3 (`0x80123456` → `0x80`) |
| 5 | X888 | `0xFF` |
| 6 | 8-bit | the texel itself |

The RGB bits never matter. For f = 2, 3, 5 the texel is read only for the key test (odd rows); in the even rows
those `Blt` / `ConstantAlpha` slots are the fill bodies and read nothing.

### 7.3 Columns (kind & 3)

Let `ca` = byte 3 of `b+0x14`.

- **Blt** (0, 4, 8): store `A`.
- **ConstantAlpha** (1, 5, 9): store `0xFF`, whatever the texel and `ca`.
- **Modulate** (2, 6, 10): store `(A × ca + 128) >> 8` (MMX; in the 8-bit-source bodies the texel is widened to the dword `A<<24 | 0x00FFFFFF`,
  `pmullw` with the unpacked colour, `paddw` `0x0080` (`0x00AC1B80`), `psrlw 8`, `packuswb`, top byte kept; the
  result never exceeds 254, so the saturation never acts). Examples: opaque source (`A = 0xFF`) with `ca = 0xFF` →
  **`0xFE`**, not `0xFF`; `ca = 0x80` → `0x80`; 8-bit texel `0x40`, `ca = 0xC0` → `0x30`.
- **ModulateAlpha** (3, 7, 11): store `0xFF`. The bodies compute the Modulate value and **discard it** (e.g.
  `0x00863F20` stores the immediate `0xFF`).
- **Fill bodies.** `F` (`0x0085FCB0`) and `T` (`0x00861BB0`) store `0xFF` over every span pixel and read neither the
  source nor the destination; `T` still evaluates `(start.x + u0) mod W` per span (unsigned `div`), so `W = 0`
  faults there even though no texel is read.

### 7.4 Key (odd rows)

With the key flag set, the pixel is **left untouched** when `(texel & mask_f) == key`, comparing against the full
32-bit key dword: masks `0x0FFF` (A4444), `0x7FFF` (A1555, X555), `0xFFFF` (R565), `0x00FFFFFF` (A8888, X888). A
key with bits outside the mask therefore never matches (the caller always passes it masked). Otherwise the column
rule of §7.3 applies — including `ConstantAlpha` / `ModulateAlpha`, which store `0xFF` only on non-keyed pixels.
Row 13 (8-bit, keyed; unreachable) compares **no texel bits**: it tests `key ≠ 0`, drawing every pixel when the key
is non-zero and none when it is 0.

### 7.5 Addressing (columns 0–3 plain, 4–7 stretch, 8–11 tiled)

- **Plain**: texel (x, y), x and y the absolute destination pixel (the caller has biased the source pointer):
  address `src + y·stride + x·bpp`, bpp 2 for f ≤ 3, 4 for f = 4, 5, 1 for f = 6.
- **Stretch**: texel (`(u0 + du·x) >> 16`, `(v0 + dv·y) >> 16`), 32-bit wrapping sums, **logical** shifts (no
  `sar` occurs anywhere in the owner range). No bounds check.
- **Tiled**: per span `sx = ((start.x + u0) mod 2³²) mod W`, `sy = ((y + v0) mod 2³²) mod H` (unsigned `div`, 82 in
  the owner), then `sx` steps by 1 and wraps to 0 at W. `W = 0`, or `H = 0` in a body that reads a texel, faults.
  Row 13 tiled `ConstantAlpha` (`0x00864940`) divides by W only.

## 8. Proofs and how to rerun them

Scripts in `docs/specs/proofs/` (need `unicorn` — `pip install --target <worktree>/build-runs/pylib unicorn` is
picked up automatically — and the GOG 1.1.641 `SimCity4.exe`; `SC4EXE` overrides the path). They load the PE image
into the emulator and run the shipped bytes against the rules above (model: `span8_model.py`).

| Script | What it checks | Result (2026-10-21) |
|---|---|---|
| `span8_emu.py` | prints the 14×12 table at `0x00AC26B0` | matches §7 |
| `span8_methods.py 300` | §2–§5 on random spans, colours and full-range gradient records; bytes popped; RenderBlt reaches `rows[key + 2f][kind]` (key flag read as a byte) | 5/5 PASS 300 |
| `span8_blt.py 300` | all 168 slots: random sources, colours, keys (incl. near-miss texels and unmasked keys), strides, stretch steps, tiled origins (incl. 32-bit u0/v0) | 168/168 |
| `span8_blt.py via 100` | the same through RenderBlt with records by pointer | 168/168 |
| `span8_sweep.py 4` | every 16-bit texel (256 for 8-bit, 65 536 random 32-bit with alpha 0/255) per slot, 4 colours | 168/168 |
| `span8_alias.py 200` | source aliased onto the destination (§1 overlap rule) | 168/168 |
| `span8_bridge.py` | §10 on a constructed type-2 fixture: buffer Init stride/size, colour map = 0 (both classes), context Init renderer per type 0–11 (fail paths leave the lock), destination record, SetColorTable, DrawPreclippedSpanList solid/alpha writes 0 (heap alloc/free stubbed) | PASS |

## 9. Open points

- **SR8-1** answered 2026-10-21 (§10.4): every shipped `cIGZBuffer` maps any RGB to **0** for colour type 2, so
  through `SetColorTable` the solid byte is always `0x00`.
- **SR8-2** (whole spec), narrowed by §10.6: the factory accepts any `cIGZBuffer`, and the only literal type-2
  buffer creation found (`cSC4WinRegionScreen::MakeWholeRegion`) is never bound to a draw context. Still open: whether
  any buffer whose type comes from data (resource loaders, screen buffer) reaches `CreateDrawContext` as type 2.
  The gradient paths additionally inherit SR565-1 (no direct caller of `SetGradientTransform`).
- Behaviour with records the caller never produces (stretch reads past the texture, `W = 0`, row 13, `n = 0`) is
  stated above for completeness but has no compatibility weight.
- **SR9** answered 2026-10-22: §§10.1, 10.5, 10.7 specify the native boundary and the bounded ABI proposal;
  applying the absent keys remains a coordinator review/train prerequisite before bridge calls are enabled.
- **SR10** is the coordinator's glue-scope clarification, not a missing native-game contract: the new checked
  C adapter is our interface design. It supplies no native ABI approval, runtime evidence or replacement credit.

## 10. Bridge: object, factory, target buffer, dispatch [exact]

Read from the 8-bit path's own instructions (`Init` entry `0x008307D4`, vtable `0x00AC188C`, the type-2 arms of
the buffer methods); no sibling renderer was used as evidence.

### 10.1 The renderer object and its vtable

The six entries were re-read as little-endian dwords from the native image for SR9 (2026-10-22).
All calls use MSVC thiscall, ECX = `this`; stack arguments are pushed right to left. Slot 4 is a tail jump
(`0x0085E742`) to slot 2, so its 16-byte cleanup is inherited, not a separate return instruction. Cleanup
instructions for slots 0, 1, 2, 3 and 5 are at `0x0082FF1C`, `0x0085E696`, `0x0085E70F`, `0x0085E739`
and `0x0085E7DF`. Only the deleting destructor promises an EAX result; render scratch values are not results.

The object is **4 bytes: the vtable pointer only**, no fields. `this` is used only by §3 and §5 to re-dispatch
through the vtable (so a subclass overriding slot 1 or 2 would change them; nothing in the shipped program does).

| Slot (offset) | Function | Arguments after `this` (`thiscall`, ECX = `this`) | Callee pops | Return |
|---|---|---|---|---|
| 0 (`+0x00`) | `0x0082FF00` deleting destructor | flags (dword; bit 0 = free) | 4 | `this` in EAX |
| 1 (`+0x04`) | RenderSolid §2 | spans, n, dst*, colour | 0x10 | none (EAX undefined, never read) |
| 2 (`+0x08`) | RenderGradient §4 | spans, n, dst*, g* | 0x10 | none |
| 3 (`+0x0C`) | RenderAlpha §3 | spans, n, dst*, colour, argb | 0x14 | none |
| 4 (`+0x10`) | RenderAlphaGradient §5 | spans, n, dst*, g* | 0x10 | none |
| 5 (`+0x14`) | RenderBlt §6 | spans, n, dst*, src*, b*, kind | 0x18 | none |

The destructor stores the base vtable `0x00AC17AC` (`cISpanRender`) into the object, then, if bit 0 of its
argument is set, frees the 4 bytes with the global heap free (`0x005E5620`). Nothing else is released.

### 10.2 Lifetime (owner: `cGZDrawContextRenderToBuffer`, `ctx+0xB0`)

- **Created** only by the context's `Init` (`0x00830710`, ctx vtable `0x00AC17D0` slot 3), after the target is
  queried and locked (§10.3): 4 bytes from the global heap (`0x005E55E0`), vtable `0x00AC188C` stored, pointer
  stored in `ctx+0xB0`. If the allocation fails, `ctx+0xB0` = 0 and `Init` **still returns success**; the next draw
  then calls through a null pointer. *[compat: crash in the original; a reimplementation may fail Init instead]*
- **Destroyed** by `Shutdown` (`0x0082FF20`, ctx slot 4): `ctx+0xB0` deleted via slot 0 with flags 1 and set to 0;
  then, if the target exists and the locked flag `ctx+0x75` is set, the target is unlocked with `0x8040`; then the
  target reference is released and `ctx+8` = 0. `Init` calls `Shutdown` first, so re-initialising replaces the
  renderer. The final `Release` (`0x008306C0`, refcount reaches 0) deletes `ctx+0xB0` the same way, releases the
  texture `ctx+0x24` and the target `ctx+8`, **without unlocking** the target.
- The constructor (`0x00830690`, not a Ghidra function) zeroes `+4` (refcount), `+8`, `+0x24`, `+0xB0` and the
  bytes `+0x41`, `+0x43`, `+0x74`, `+0x75`; it is called only by the software context factory (§10.5).

### 10.3 `Init(target)` and factory selection

`Init` takes one argument (`ret 4`) and returns a bool in AL. Steps, in order:

1. `Shutdown` (§10.2); release `ctx+8` again if still set (it never is after Shutdown).
2. `target->QueryInterface(0x20732180 = cIGZBuffer, &ctx+8)`; failure → return false.
3. `buffer->Lock(0x8040)` (vfunc `+0x18`); failure → release the buffer, `ctx+8` = 0, return false.
4. `buffer->GetColorType` (vfunc `+0x3C`, writes the pair {colour type, bits per pixel}); the type selects the
   renderer through the jump table `0x008308B0` (index `type − 2`, unsigned, valid 0–7):

| Colour type | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | other |
|---|---|---|---|---|---|---|---|---|---|
| vtable | **`0x00AC188C` (this class)** | fail | `0x00AC182C` | `0x00AC1844` | `0x00AC182C` | `0x00AC185C` | `0x00AC1874` | `0x00AC1874` | fail |

   The bits-per-pixel half is **not** consulted: a type-2 buffer gets the 8-bit renderer whatever its bpp says.
   "fail" releases the buffer, sets `ctx+8` = 0 and returns false, but **leaves the buffer locked** (the lock of
   step 3 is never undone). *[compat]*
5. Copies the destination record `ctx+0x78` (§1 layout) from the buffer: `+0x78` = pixel pointer (vfunc `+0x88`),
   `+0x7C` = stride (`+0x8C`), `+0x80` = width (`+0x24`), `+0x84` = height (`+0x28`), `+0x88` = colour type,
   `+0x8C` = bytes per pixel (`+0x44`); also `ctx+0x0C` = width, `ctx+0x10` = height, `ctx+0x90` = bytes per pixel
   (`+0x44` again), `ctx+0xA4` (tiling) = 0, `ctx+0x75` (locked) = 1; returns true.

The pixel pointer and stride are captured **once**: the buffer stays locked for the whole life of the context, and
the record is not refreshed per draw.

### 10.4 The type-2 target buffer (`cGZBuffer`, CLSID `0xC470D325`, vtable `0x00AC1400`)

- **Init** (vfunc `+0x0C`, `0x008269B0`; width, height, colour type, bits per pixel; `ret 0x10`): fails if already
  initialised (byte `+8`) or width or height is 0; stores width `+0x1C`, height `+0x20`, type `+0x0C`, bpp `+0x10`,
  then allocates storage (`0x00826A50`): row stride = `(width × (bpp >> 3) + 3) & ~3` (rows padded to 4 bytes),
  `stride × height` bytes from the global heap, not cleared here; pointer `+0x3C`, stride `+0x40`, bytes per pixel
  `bpp >> 3` (vfunc `+0x44` = `bpp >> 3` of `+0x10`). Example: Init(10, 3, 2, 8) → stride 12, 36 bytes, 1 byte per
  pixel. A type-2 buffer is just this layout with 1 byte per pixel and **no palette**: the storage set-up
  (`0x00826390`, vfunc `+0xB4`) fetches a palette service only for types 1 and 3, so type 3 is the paletted 8-bit
  format (rejected by the context, §10.3) and type 2 the unpaletted one.
- **Lock** (`+0x18`, `0x00826AA0`, flags; `ret 4`): false if there is no storage; otherwise increments the 16-bit
  lock count `+0x38`, ORs the flags into `+0x44`, and if flag bit 15 (`0x8000`) is set also increments the change
  counter `+0x30`; true. **Unlock** (`+0x1C`, `0x00826490`): decrements the 16-bit count, clears only the supplied
  flag bits from the 32-bit accumulated mask; always true, even at zero nesting (GZF10, framework core §6).
  The context's `0x8040` therefore marks the buffer as modified once per `Init`, not per draw.
- **Colour mapping** (`+0x78`, `0x00825DA0`, r, g, b as the low bytes of three dwords; `ret 0xC`): switch on the
  colour type with table `0x00825E60` over types 4–9; any other type, **including 2**, returns **0**. The second
  buffer class with the same interface (vtable `0x00AC1550`, mapping `0x0082AD30`, type at `+0x2C`) also returns 0
  for type 2, and the screen-buffer vtable `0x00ADB418` reuses `0x00825DA0`. Hence (SR8-1) `SetColorTable`
  (`0x0082FF70`: for each entry `i`, raw ARGB → `ctx+0xB4+4i`, `buffer->+0x78(argb>>16, argb>>8, argb)` →
  `ctx+0xC4+4i`) always yields `ctx+0xC4 = 0`, and RenderSolid / RenderAlpha on a type-2 target store **`0x00`**,
  whatever colour was set. Only the blit (§7, alpha from the source texel and `ctx+0xB4` byte 3) and gradient (§4)
  paths can produce non-zero bytes. *[compat: solid fills on an 8-bit target clear to 0]*

### 10.5 How a context reaches this renderer (COM route)

1. `cGZDrawSystem::Init` (`0x0082BC90`) creates the software context factory (12 bytes, vtable `0x00AC162C`,
   interface IID `0x6B2EA76C` per its QueryInterface `0x0082B850`) and appends it to its factory list
   (`drawSystem+0x1C`).
2. `cIGZDrawSystem::CreateDrawContext(target, &ctx)` (vtable `0x00AC1668` slot 4, `0x0082BAB0`) asks each factory
   in list order (`+0x0C`), first success wins. The software factory (`0x0082B910`) succeeds for any target that
   answers `cIGZBuffer`: it allocates `0xD4` bytes, constructs (§10.2), AddRefs and returns the context
   **uninitialised** (no `Init` call; null allocation is dereferenced, *[compat: crash]*). The other registered
   factory (vtable `0x00ADB4D8`, `0x00993270`) needs interface `0xE3094DC8` and builds a different context.
3. The caller then calls `ctx->Init(target)` (§10.3); with a type-2 buffer this installs vtable `0x00AC188C`.
4. Primitives: the context's `Draw` (`0x00831000`, ctx vtable `+0x54`) handles only antialiased primitives (types
   6, 7, 8 with the antialias byte `ctx+0x43` set) through `FillAntialiasedPolygon` (`0x008308D0`), which writes the
   target **directly** through the buffer's set-pixel vfunc `+0x58` (`0x00826560`; 1 byte per pixel → stores the low
   byte of the value) and **never calls this renderer**; otherwise it returns 2 / 1 and the generic rasteriser
   `cGZDrawContext` (`RasterizeFillTri` `0x0082E430`, `RasterizeFillQuad` `0x0082E5C0`,
   `RasterizeFillPolygon` `0x0082E740`) produces clipped span lists and calls `DrawPreclippedSpanList` (ctx vtable `+0x58`,
   `0x008302E0`, `thiscall`, spans and n, `ret 8`, no return value) on its render context (`drawContext+0x28`).
   `DrawPreclippedSpanList` picks the renderer slot:

| Context state | Slot called on `ctx+0xB0` | Extra arguments |
|---|---|---|
| no texture, gradient flag `+0x74` set, alpha flag `+0x41` set | 4 RenderAlphaGradient | `&ctx+0x78`, `&ctx+0x44` |
| no texture, gradient, no alpha | 2 RenderGradient | `&ctx+0x78`, `&ctx+0x44` |
| no texture, no gradient, alpha | 3 RenderAlpha | `&ctx+0x78`, `ctx+0xC4`, `ctx+0xB4` |
| no texture, no gradient, no alpha | 1 RenderSolid | `&ctx+0x78`, `ctx+0xC4` |
| texture `ctx+0x24` set and its `Lock(0x800)` succeeds | 5 RenderBlt | `&ctx+0x78`, src record, b record, kind |

   The texture branch is the one of `span-render-565.md` §6.1 (format index, key masking, kind bits, tiling and
   stretch origins), unchanged for this target; it unlocks the texture with `0x800` after the call. A texture of an
   unsupported colour type, or a failed texture lock, draws nothing. `ctx+0xB0` is never null-checked.

**Slot-22 return contract (SR9):** void, popping 8 bytes. Its eight exits (`0x008304E7`, `0x00830551`,
`0x008305A7`, `0x008305BC`,
`0x008305EF`, `0x00830604`, `0x0083062F`, `0x0083064F`) have identical cleanup but no common return-value
definition: texture success/unsupported format leaves Unlock scratch, lock failure leaves Lock scratch,
and untextured paths leave renderer scratch. The triangle rasterizer calls it at `0x0082E5AA` and proceeds
directly to restoring registers. The quad and polygon callers likewise ignore it (`0x0082E72A`,
`0x0082E881`). Do not promote any incidental AL/EAX contents to a bool or count.

Worked route (constructed fixture): create a `cGZBuffer`, `Init(8, 2, 2, 8)` (stride 8); `CreateDrawContext` →
`Init` → renderer `0x00AC188C`, destination record {pointer, 8, 8, 2, 2, 1}; `SetColorTable(0, 1, {0xFF336699})`
gives `ctx+0xB4 = 0xFF336699`, `ctx+0xC4 = 0`; a solid span (0,0)–(5,0) writes `00 00 00 00 00` at bytes 0–4 of row
0. With an 8-bit alpha texture (format 6) bound, plain kind 0, the same span writes per pixel the §7.2 alpha of each
texel instead.

### 10.6 Shipped producers of type-2 buffers (SR8-2, partial)

- The only literal `Init(…, 2, 8)` call found is in `cSC4WinRegionScreen::MakeWholeRegion` (`0x007AEDD0`): a square
  `0x100 >> (2 − size)` heightfield buffer filled pixel by pixel through the buffer's own set-pixel vfunc (`+0x58`)
  under `Lock(0x8080)`; no draw context is created on it, so it never reaches this renderer.
- Buffers whose type comes from data (`cGZBufferResource` loading with the factory's `SetColorType`, the persist
  buffer factory, the screen buffer) were not traced to a `CreateDrawContext` call; that is what remains of SR8-2.
- Distinguish: §8's proofs and the worked route above are **constructed fixtures**; no shipped draw into a type-2
  target is known.

### 10.7 SR9: bounded native ABI evidence and reconciliation [exact, static]

All rows here are **MSVC thiscall**: ECX holds the object/interface pointer, stack arguments occupy dword
slots and are pushed right to left; Pops excludes the return address. EAX means a full 32-bit value or pointer,
AL means only the low byte is defined, and void promises no result. Names are descriptive; the exact ABI key is
`(interface, game_class, vtable, slot)`, never the method name or a shared function's export label.

**Provenance.** Read the vtable dwords directly from GOG 1.1.641's native executable, SHA256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, and compare the queried instructions'
bytes with that image: **1,782 instructions in 43 function ranges match** (the 40 proposed targets and three
rasterizer callers). This is static verification, not an executed native fixture or a game/shadow run. The export
does not decode the software factory entry `0x0082B910`, and the local fallback lacks capstone; §10.5's existing
factory proof is retained, without adding a factory ABI row or claiming a new audit of that entry. REA was not used.

**Context and draw system.** Context rows below have interface `cIGZDrawContext`, game class
`cGZDrawContextRenderToBuffer`, vtable `0x00AC17D0`. The shared public setter behaviour is
[span-render-565.md §1.3](span-render-565.md#13-context-public-interface-used-by-the-route-re-span565-context4-exact-static)
and §6.2: Init selects this renderer for type 2, but the setters and slot-22 consumer are the same bytes.
Colour table packing is through the target's MapRGB; for type 2 its result is zero (§10.4).

| Slot | Target | Method; arguments after this | Pops | Result | Cleanup evidence |
|---|---|---|---|---|---|
| 0 | `0x0082FD60` | QueryInterface; iid, out | 8 | AL | `0x0082FD70`, `0x0082FD80` |
| 1 | `0x005F47F0` | AddRef; none | 0 | EAX new count | `0x005F47F7` |
| 2 | `0x008306C0` | Release; none | 0 | EAX remaining count | `0x0083070C` |
| 3 | `0x00830710` | Init; target | 4 | AL | `0x00830877`, `0x00830892`, `0x008308AB` |
| 4 | `0x0082FF20` | Shutdown; none | 0 | void | `0x0082FF6A`; tail call at `0x0082FF66` to target Release also pops 0 |
| 7 | `0x0082FF70` | SetColorTable; index, count, argb* | 12 | void | `0x0082FFC0` |
| 8 | `0x0082FEA0` | SetAlphaBlend; signed int | 4 | void | `0x0082FEAC` |
| 9 | `0x0082FEB0` | SetBltModulate; low byte in dword slot | 4 | void | `0x0082FEB7` |
| 12 | `0x0082FDB0` | SetGradientTransform; float[12]* or null, binary32 x0, y0 | 12 | void | `0x0082FE4D`, `0x0082FE58` |
| 13 | `0x0082FFD0` | SetBltSource; texture, binary32 x, y, sx, sy, outRect* | 24 | AL | `0x00830278`, `0x00830296` |
| 14 | `0x0082FE60` | ClearBltTiling; none | 0 | void | `0x0082FE67` |
| 15 | `0x0082FE70` | SetBltTiling; rect* | 4 | void | `0x0082FE97` |
| 22 | `0x008302E0` | DrawPreclippedSpanList; spans, n | 8 | void | eight exits in §10.5 |

SetAlphaBlend stores 1 only for a signed argument greater than zero; SetBltModulate stores the passed low byte,
without normalising it. Null gradient input clears the gradient flag but leaves its record unchanged; a null
texture releases the previous source and returns AL = 0. A nonnull texture whose QueryInterface fails follows
the AL = 1 exit with no bound source (`0x00830008` → `0x00830270`), as already specified in the public setter
proof. No bounds checks are added to SetColorTable. Init does not initialise all drawing state: use the setters
before the constructed draw (§10.5), as the 565 proof's fixture does.

Draw-system row: interface `cIGZDrawSystem`, game class `cGZDrawSystem`, vtable `0x00AC1668`, **slot 4**
(`+0x10`) → `0x0082BAB0`, CreateContext(target, outContext), Pops **8**, result **AL**. Empty/all-refused
factory list returns 0; first successful factory returns 1 (`0x0082BADF`, `0x0082BAE8`). The result is tested
as AL after the factory call at `0x0082BACC`. The list head is `[this+0x20]`; §10.5's `+0x1C` names the list
storage, not the sentinel pointer used by this walk. The context still needs its separate Init call.

**Buffer calls used by this route.** All following rows have interface `cIGZBuffer`, game class `cGZBuffer`,
vtable `0x00AC1400`; none is inferred from another buffer's vtable. Descriptor output pointers and integer
returns are full EAX values even when a given caller ignores them.

| Slot | Target | Method; arguments after this | Pops | Result | Cleanup evidence |
|---|---|---|---|---|---|
| 0 | `0x00825D00` | QueryInterface; iid, out | 8 | AL | `0x00825D19`, `0x00825D30`, `0x00825D42`, `0x00825D52` |
| 1 | `0x00825D60` | AddRef; none | 0 | EAX new count | `0x00825D67` |
| 2 | `0x00825D70` | Release; none | 0 | EAX remaining count | `0x00825D8B`, `0x00825D90` |
| 3 | `0x008269B0` | Init; width, height, type, bpp | 16 | AL | `0x008269FA`, `0x00826A00` |
| 6 | `0x00826AA0` | Lock; flags | 4 | AL | `0x00826AC0`, `0x00826AC5` |
| 7 | `0x00826490` | Unlock; flags | 4 | AL | `0x008264A4` |
| 9 | `0x00808620` | GetWidth; none | 0 | EAX `[this+0x1C]` | `0x00808623` |
| 10 | `0x004ED900` | GetHeight; none | 0 | EAX `[this+0x20]` | `0x004ED903` |
| 11 | `0x008268D0` | GetRect; outRect* (16 bytes) | 4 | EAX = outRect | `0x008268ED` |
| 15 | `0x008268F0` | GetColorType; outPair* (8 bytes) | 4 | EAX = outPair | `0x008268FF` |
| 17 | `0x00826910` | GetBytesPerPixel; none | 0 | EAX bpp logically shifted right by 3 | `0x00826916` |
| 26 | `0x00826620` | GetColorKey; outKey* (4 bytes) | 4 | AL enabled | `0x00826631`, `0x00826636` |
| 30 | `0x00825DA0` | MapRGB; r, g, b low bytes in three dwords | 12 | EAX packed colour (0 for type 2) | `0x00825DE4`, `0x00825E0E`, `0x00825E3C`, `0x00825E5D` |
| 34 | `0x008265C0` | GetPixels; none | 0 | EAX `[this+0x3C]` | `0x008265C3` |
| 35 | `0x0068D1B0` | GetStride; none | 0 | EAX `[this+0x40]` | `0x0068D1B3` |
| 40 | `0x00827340` | DeletingDestructor; flags | 4 | EAX = this | `0x00827379` |
| 42 | `0x00826350` | AllocateStorage; none | 0 | AL from slot 43 | `0x00826367`, `0x0082636B` |
| 43 | `0x00826A50` | AllocatePixels; none | 0 | AL = 1 | `0x00826A95` |
| 44 | `0x00826370` | FreePixels; none | 0 | AL = 1 | `0x00826389` |
| 45 | `0x00826390` | ResolveColorType; none | 0 | void | `0x00826431`, `0x0082644F`, `0x00826470`, `0x00826485` |

QueryInterface accepts iid 1 or `0x20732180` at the primary pointer (writes this and AddRefs); the additional
`0x86D72B57` interface uses this+4 and is not invoked through this table in the route. AddRef/Release use the
count at +0x2C; final Release calls deleting-destructor slot 40 with flags 1 and then returns zero. The destructor
returns the original pointer even after freeing it; that is not a live reference. GetRect copies +0x14..+0x20
(fresh Init gives 0,0,W,H); GetColorType copies +0x0C and +0x10 (type,bpp), not a scalar type. Its returned pointer
is dereferenced by context Init (`0x00830767`) and the texture consumer (`0x0083032B`). GetColorKey tests bit 1
at +0x28: enabled writes +0x24 to outKey and returns 1; disabled leaves outKey untouched and returns 0.

The direct call sites delimit the proposal: Init uses buffer QueryInterface, Lock and destination descriptors
(§10.3); SetColorTable invokes slot 30 (`0x0082FFA9`); SetBltSource invokes QueryInterface/Release and slot 11
(`0x00830017`); slot 22 invokes source descriptor slots 9/10/15/17/34/35 (`0x0083030B`..`0x00830354`), key
slot 26 (`0x008303BC`) and Unlock (§10.5). Shutdown and final Release release their buffer references (§10.2).
Context QueryInterface/AddRef/Release are included for acquiring and disposing the returned context.

The necessary buffer lifecycle closure is Init → slot 42 (`0x008269EA`) → slot 43 (`0x00826355`) → slot 45
(`0x00826A57`), and Release → slot 40 (`0x00825D83`) → slot 44 (`0x00827350`, indirect through table address
`0x00AC14B0`). ResolveColorType leaves a type-2 descriptor unchanged (it only resolves types 1 and 3).
AllocatePixels returns AL = 1 even if the heap allocation returned null; subsequent Lock then fails. Neither
that incidental success nor the destructor's pointer result makes an allocation failure safe to draw through.
No additional methods for antialiased direct-pixel drawing, other buffer classes or palette-service interfaces
are proposed. This is a bounded span8 fixture route, not a complete interface catalogue.

**Exact-key reconciliation.** On 2026-10-22 the main checkout's `docs/abi/interfaces.tsv` contains **0 of these
40 keys**: 6 renderer, 13 context, 1 draw system and 20 buffer rows. The earlier 17-row
`docs/abi/proposals/re-span-context4.tsv` overlaps **14** keys: nine public context setters/lifecycle rows,
slot 22, three buffer rows and draw-system slot 4. Thirteen overlapping contracts match; its slot-22
`unknown` is strengthened to **void** by §10.5, preserving the stronger existing public spec. The remaining
three earlier rows (context Suspend, Resume, LockTarget) are outside this bounded route and remain untouched.
No existing applied row is replaced, and no unknown pop count or return is invented.

The lane publishes the 40 absent rows, using the existing 12-column interfaces schema, at
`build-runs/abi-rows-re-span8-abi4.tsv` in worktree `re-span8-abi4`. The coordinator must review by the complete
key again at application time, preserve any stronger intervening answers, and apply only approved absent keys
through the gated train. **The proposal is not applied ABI approval:** no table, ABI version, native bridge or
C export changes are made here. SR9's native contract uncertainty is resolved; bridge enablement still waits
for that review/application and the SR10 glue adapter. SR8-2 reachability and SR8-3 row-13 restrictions remain.
