# Span renderer, 16-bit 1555 (`nGZDrawSoftRender::cSpanRender1555`)

Owner range `0x008310B0`–`0x0083DE50` (172 functions, 50 817 bytes; `docs/specs/view3d.md` §7.2). This is the
span back end that `cGZDrawContextRenderToBuffer::Init` (`0x00830710`) creates into `+0xB0` for buffer colour
types **4 (`R5G5B5`) and 6 (`A1R5G5B5`)**: one class serves both, so nothing here distinguishes "X" from "A"
1555 targets. The 4444, 565, 8888 and 8-bit siblings are specified elsewhere (4444: its own lane's file).

Status legend used below: **[exact]** = every instruction of the function read and restated; **[table]** = the
dispatch slot is identified but the instantiation's per-pixel body is not yet restated.

Pixel layout (destination word, little-endian `u16`): bits 0–4 **B**, 5–9 **G**, 10–14 **R**, bit 15 **A**.
Masks used by the code: `0x001F`, `0x03E0`, `0x7C00`, `0x8000`, and the paired `0x7C1F` (R+B) / `0x03E0` (G).

## 1. Common inputs

- **Span list.** An array of `n` span records, 8 bytes each: `+0` start, `+4` end. Each is a packed point,
  **x in the low 16 bits, y in the high 16 bits** (unsigned). The pixel count is `end − start` computed as a
  32-bit difference, so a span is valid only when both points share a row (`end.y == start.y`, `end.x ≥ start.x`);
  pixels `x = start.x … end.x − 1` are written, **left to right**, end-exclusive. A zero-length span writes nothing.
- **Span count `n`.** All five methods loop `do { … } while (--n)`: **`n = 0` is not a no-op** (it underflows and
  walks 2³² records). Callers must never pass 0. *[compat: never pass 0; the original's behaviour is a crash]*
- **Surface.** A two-dword record: `+0` base pointer of pixel (0, 0), `+4` row stride in **bytes** (signed use:
  `y × stride` is a 32-bit multiply). Pixel address = `base + y × stride + 2 × x`. No clipping is done in this
  class: spans arrive pre-clipped (`DrawPreclippedSpanList`, `0x008302E0`), and no bounds are read here.
- **Overlap.** Spans are processed in list order, each fully before the next; a pixel covered twice is blended
  twice (the second blend reads the first's result). RenderBlt reads a separate source; see §6.

## 2. RenderSolid (`0x008310B0`, slot 1) [exact]

Arguments: spans, n, surface, colour (dword). For each span it calls the 16-bit fill helper `0x0090FFC8`
(dest, value, count), which writes the **low 16 bits of `colour` verbatim** to `count` consecutive pixels
(including bit 15: the caller's packed value decides A). The helper writes one leading word when the destination
is not 4-byte aligned, then dword pairs, then a trailing word; the observable result is just `count` copies.
`count = 0` writes nothing.

## 3. RenderAlpha (`0x00831200`, slot 3) [exact]

Arguments: spans, n, surface, `src` (dword; low 15 bits are a packed 1555 colour, bit 15 ignored), `argb`
(dword; only bits 24–31, the 8-bit alpha `a8`, are used). Five arguments (`ret 0x14`).

1. **Weight.** `w = (a8 × 0x818180) >> 26` (32-bit unsigned), giving `w ∈ 0…32`. Thresholds (first `a8` that
   gives each `w`): 1:8 2:16 3:24 4:32 5:40 6:48 7:56 8:64 9:72 10:80 11:87 12:95 13:103 14:111 15:119 16:127
   17:135 18:143 19:151 20:159 21:167 22:174 23:182 24:190 25:198 26:206 27:214 28:222 29:230 30:238 31:246
   32:254. So `a8 = 254` and `255` both give full replacement; `a8 ≤ 7` gives no colour change.
2. **Per pixel**, with `S`, `D` the 5-bit source and destination channel values:
   `C_out = floor(((32 − w) × D + w × S + 16) / 32)` for each of R, G, B independently.
   The code does R and B in one 32-bit multiply on the `0x7C1F` pair with rounding constant `0x4010`, and G on
   `0x03E0` with `+0x10` (pre-shift); the sums never carry between channels (max 1008 < 1024), so the per-channel
   formula is exact.
3. **Alpha bit.** Output bit 15 is **always 1**, whatever `D` and `src` held (both OR masks `0x83E0`/`0xFC1F` set
   it before the AND).

Worked example: D = `0x0000` (black, A=0), src = `0x7FFF` (white), `a8 = 0x80` → `w = 16`;
each channel `floor((16×31 + 16)/32) = 16` → output `0x8000 | 16<<10 | 16<<5 | 16 = 0xC210`.

## 4. RenderGradient (`0x00831100`, slot 2) [exact]

Arguments: spans, n, surface, `g` (pointer to the gradient record, §4.1). Four arguments (`ret 0x10`).

### 4.1 Gradient record (0x30 bytes, shared with §5)

Twelve signed dwords: `+0x00..+0x0C` start values `c0, c1, c2, c3`; `+0x10..+0x1C` per-x steps `dx0..dx3`;
`+0x20..+0x2C` per-y steps `dy0..dy3`. The value of channel `i` at absolute pixel (x, y) is
`V_i(x, y) = c_i + dx_i × x + dy_i × y` (32-bit wrapping arithmetic). `c0` lands in **B**, `c1` in **G**, `c2` in
**R**, `c3` is the alpha weight (§5 only). Who fills it (expected 8.16 fixed point, 8-bit channel in bits 16–23,
as §5's conversion implies) is outside this class: question SR1555-1.

### 4.2 Per-pixel rule

For a span starting at (x0, y), pixel `k = 0, 1, …` (x = x0 + k):

- `B = (A0 >> 16) & 31` with `A0 = V_0(x0, y) + k × dx0`;
- `G = (A1 >> 21) & 31` with `A1 = V_1(x0, y) + k × (dx1 << 5)`;
- `R = (A2 >> 26) & 31` with `A2 = V_2(x0, y) + k × (dx2 << 10)`;
- bit 15 = **0** (the code adds `0x80000000` to the R accumulator at the span start, which only toggles bit 31
  and never reaches the output).

All arithmetic is 32-bit wrapping; there is **no clamping**, a channel wraps modulo 32. Note the asymmetry: the
**start value is not shifted, only the step is** (×32 for G, ×1024 for R). With 8.16 input this takes B from bits
16–20 (the low 5 bits of the 8-bit value) and the G and R *start* from bits 21–25 / 26–30. The 565 sibling
(`0x0083DEA0`) shifts start and step alike (B `>>3`, G `<<3`, R `<<8`), so this is a 1555-only quirk of the
original; reproduce it bit for bit. *[compat if any shipped path draws a gradient into a 1555 buffer: SR1555-2]*

## 5. RenderAlphaGradient (`0x008312D0`, slot 4) [exact]

Arguments as §4 (`ret 0x10`). Per span starting at (x0, y), pixel k:

- source **B** = `(Ab >> 16) & 31`, `Ab = (V_0(x0, y) >>> 3) + k × (dx0 >> 3)` — start shifted **logically**,
  step **arithmetically**, each truncated separately (so the B step loses its low 3 bits every pixel; over long
  spans B can differ from `floor(V_0(x)/2^19)`);
- source **G** = `(Ag >> 21) & 31`, `Ag = (V_1(x0, y) << 2) + k × (dx1 << 2)` (= bits 19–23 of `V_1(x)`);
- source **R** = `(Ar >> 26) & 31`, `Ar = (V_2(x0, y) << 7) + k × (dx2 << 7)` (= bits 19–23 of `V_2(x)`);
- weight `w = (Aa >> 26)` (unsigned), `Aa = V_3(x0, y) × 0x84 + k × (dx3 × 0x84)` (32-bit wrap). For a
  zero-fraction 8-bit alpha `A` (`V_3 = A << 16`) the thresholds are 1:8 2:16 3:24 4:32 5:39 6:47 7:55 8:63 9:70
  10:78 11:86 12:94 13:101 14:109 15:117 16:125 17:132 18:140 19:148 20:156 21:163 22:171 23:179 24:187 25:194
  26:202 27:210 28:218 29:225 30:233 31:241 32:249 — **not** the §3 mapping (they differ from `A = 39` on).
  `w` can exceed 32 only if `V_3` leaves 0…255.99 (bits ≥ 24); then the blend below is not a lerp. *[compat]*
- blend: exactly §3 step 2 (`C_out = floor(((32 − w) D + w S + 16) / 32)`, same constants `0x4010`/`0x10`),
  and bit 15 = 1.

## 6. RenderBlt (`0x00831470`, slot 5) and the dispatch table

Arguments (six, `ret 0x18`): `p1`, `p2`, `r3` (pointer to a 24-byte record), `r4` (pointer to a 24-byte record),
`b` (pointer to a 32-byte blit record), `kind` (column index). It copies `r3`, `r4` and `b` by value and calls
`row[kind](p1, p2, *r3, *r4, *b)` (cdecl), where `row = rows[b.byte(+0x1C) + 2 × b.dword(+0x10)]` and `rows` is the
14-entry pointer array at `0x00AC1B44` (row `i` = `0x00AC1B14 − 0x30 × i`, 12 slots each, the block
`0x00AC18A4`–`0x00AC1B40`). No range check is done on either index.

The Mac build names the instantiations `cBlitter<SrcFormat, DstFormat, bool>::<method>`, and for this destination
there are exactly 7 source formats × 2 bools = 14 rows and 12 non-`BltRect` methods = 12 columns
({`Blt`, `BltConstantAlpha`, `BltModulate`, `BltModulateAlpha`} × {plain, `Stretch`, `Tiled`}). The row/column
↔ name assignment is §7.
