# Span renderer, 16-bit 1555 (`nGZDrawSoftRender::cSpanRender1555`)

Owner range `0x008310B0`–`0x0083DE50` (172 functions, 50 817 bytes; `docs/specs/view3d.md` §7.2). This is the
span back end that `cGZDrawContextRenderToBuffer::Init` (`0x00830710`) creates into `+0xB0` for buffer colour
types **4 (`R5G5B5`) and 6 (`A1R5G5B5`)**: one class serves both, so nothing here distinguishes "X" from "A"
1555 targets. The 4444, 565, 8888 and 8-bit siblings are specified elsewhere (4444: its own lane's file).

Status legend used below: **[exact]** = every instruction of the function read and restated; **[verified]** = the
rule was checked against the shipped bytes by emulation (§7.7); as of round 2 every one of the 172 functions is
[exact] or [verified].

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

### 1.1 Native call boundary (re-span1555-bridge4) [exact]

Every fact here was read from the 1555-specific bytes at the cited addresses; the shared context code is the same
code `span-render-4444.md` §1.1 cites, re-read for this class.

**Creation.** `cGZDrawContextRenderToBuffer::Init` (`0x00830710`, thiscall `(buffer)`, `RET 4`) locks the buffer
(`vfunc +0x18`, flags `0x8040`; failure → return 0), reads its colour type (`vfunc +0x3C`, first dword of the
returned pair) and switches on `type − 2` through the 8-entry table at `0x008308B0`:

| Colour type | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | other |
|---|---|---|---|---|---|---|---|---|---|
| Case | `0x008307D4` | `0x0083087A` | `0x0083077C` | `0x00830792` | `0x0083077C` | `0x008307A8` | `0x008307BE` | `0x008307BE` | `0x0083087A` |
| Renderer vtable | `0x00AC188C` (8-bit) | none: release the buffer, return 0 | **`0x00AC182C` (1555)** | `0x00AC1844` (565) | **`0x00AC182C`** | `0x00AC185C` (4444) | `0x00AC1874` (8888) | same | none |

Types 4 and 6 share one case: `GlobalHeapAlloc(4)` (`0x005E55E0`, call `0x0083077E`), vtable store
`0x0083078A` (the only occurrence of the immediate `0x00AC182C` in the image), pointer stored at context `+0xB0`
(`0x008307EE`). A failed allocation stores **0** and Init still continues and returns 1; nothing later tests
`+0xB0` for null (`0x008304AD`, `0x00830529`, `0x00830583`, `0x008305C2` load and dereference it), so the next
draw would crash. *[compat: an implementation may fail Init instead]* Init then caches the surface record
`ctx+0x78` = pixel pointer (`vfunc +0x88`), `+0x7C` = stride (`vfunc +0x8C`), with width `+0x80` (`+0x24`), height
`+0x84` (`+0x28`). The renderer never reads the colour type again: a type-4 and a type-6 context are
indistinguishable from here on, and every method assumes 2 bytes per pixel.

**Object and vtable.** 4 bytes, only the vtable pointer. `0x00AC182C` is one static table of six entries:
slot 0 `0x0082FF00` (shared by all five span vtables and the base `0x00AC17AC`), slots 1–5 the five methods below;
each slot-1…5 address occurs exactly once in the image as a dword (its own entry). Patching entries 1–5 of
`0x00AC182C` therefore redirects every type-4 and type-6 context; nothing copies the table.

**Lifetime.** `Shutdown` (`0x0082FF20`; also the first thing Init does) and `Release` (`0x008306C0`) call slot 0
with flags 1 and clear `+0xB0`. Slot 0 is thiscall `(flags)`, `RET 4`: it sets the vtable to `0x00AC17AC`, frees
`this` with `GlobalHeapFree` (`0x005E5620`) when bit 0 is set, returns `this` in EAX.

**Calling convention, slots 1–5.** MSVC thiscall: `this` in ECX and never read (each method overwrites ECX before
any read of it; the object has no state), arguments pushed right to left, callee pops, no return value (no caller
reads EAX). EBX, EBP, ESI, EDI preserved (each method pushes and pops the ones it uses; the caller keeps ESI live).

| Slot | Offset | Method | Arguments (dwords, in order) | Callee pops |
|---|---|---|---|---|
| 0 | +0x00 | deleting destructor `0x0082FF00` | `flags` | `RET 4` (`0x0082FF1C`) |
| 1 | +0x04 | RenderSolid `0x008310B0` | `spans, n, surface, colour` | `RET 0x10` (`0x008310F3`) |
| 2 | +0x08 | RenderGradient `0x00831100` | `spans, n, surface, grad` | `RET 0x10` (`0x008311F4`) |
| 3 | +0x0C | RenderAlpha `0x00831200` | `spans, n, surface, src, argb` | `RET 0x14` (`0x008312C0`) |
| 4 | +0x10 | RenderAlphaGradient `0x008312D0` | `spans, n, surface, grad` | `RET 0x10` (`0x0083146D`) |
| 5 | +0x14 | RenderBlt `0x00831470` | `spans, n, surface, src, b, kind` | `RET 0x18` (`0x008314FF`) |

`spans`, `surface`, `grad`, `src` (slot 5) and `b` are pointers; `n`, `colour`, `src` (slot 3), `argb` and `kind`
are values. RenderBlt reads `b.+0x10`/`b.+0x1C` before copying, pushes copies of `b` (32 bytes), `src` (24) and
`surface` (24) by value, then `n` and `spans`, calls `rows[…][kind]` (cdecl, 22 dwords) and pops them itself
(`ADD ESP,0x58` at `0x008314FA`). RenderSolid's per-span fill helper `0x0090FFC8` is cdecl `(dest, colour, count)`,
popped by the caller (`ADD ESP,0xC` at `0x008310E9`).

**The only native caller** is `DrawPreclippedSpanList` (`0x008302E0`, thiscall `(spans, n)`, `RET 8`, slot 22 of
the context vtable `0x00AC17D0`), which passes its own `spans`, `n` through unchanged, `surface = ctx+0x78`
always, and picks one slot per call (§6.1): slot 5 at `0x008304D1`, `0x0083053B`, `0x00830591` (textured; one
site ORs 4 into `kind` at `0x0083050F`), slot 4 at `0x008305E7`, slot 2 at `0x008305FC`, slot 3 at `0x00830627`,
slot 1 at `0x00830647`. The textured path first locks the texture (`vfunc +0x18`, `0x800`, `0x008302FB`; failure →
return without drawing) and unlocks it (`vfunc +0x1C`) after the call.

**Supported COM boundary.** No interface slot returns or sets `+0xB0`. The only way to get a
`cGZDrawContextRenderToBuffer` is `cIGZDrawSystem` slot 4 (`0x0082BAB0`, thiscall `(buffer, out)`, `RET 8`),
which tries the registered context factories front to back; the RenderToBuffer factory (`0x0082B910`, slot 3 of
`0x00AC162C`) accepts any object answering `cIGZBuffer` `0x20732180`, allocates 0xD4 bytes, constructs
(`0x00830690`) and AddRefs into `*out`; it does not call Init. Init has no direct caller: its only reference is
slot 3 (`+0x0C`, entry `0x00AC17DC`) of the context vtable `0x00AC17D0`, so the holder of the context calls
`Init(buffer)` through the interface before drawing. A plugin therefore
reaches the 1555 methods either through a draw context on a type-4/6 buffer it owns, or by patching
`0x00AC182C` entries 1–5. *[same shared factory path as `span-render-4444.md` §1.1; re-read here]*

**Conformance examples (boundary).** (1) Slot 1, `spans = [{start 0x00010002, end 0x00010005}]`, `n = 1`,
`surface = {base B, stride 8}`, `colour = 0x12347C1F`: words at `B+12`, `B+14`, `B+16` become `0x7C1F`, nothing
else changes, 16 bytes popped. (2) Slot 5 with `b.+0x10 = 0` (A4444), `b.+0x1C = 0`, `kind = 0`, one span of one
pixel whose texel is `0xF8C3`: the destination word becomes `0xC726` (§7.3), via `0x00831510`.

### 1.2 Which shipped buffers reach this class (re-span1555-bridge4) [static; runtime open as SR1555-6]

A 1555 renderer exists only for a buffer of colour type 4 or 6 that gets a RenderToBuffer context. Static sweep
of the image (`build-runs/scan.py`):

| Source | Result |
|---|---|
| The 34 direct calls of the colour-pair constructor `0x0044A490` | immediate types 1 (11 calls), 9 (4), 3 (10), a register/memory type (7) and two with other pushes; **no 4 or 6** |
| The 37 places where `PUSH 4`/`PUSH 6` precedes a `CALL [reg+0x14]` within 16 bytes | all in UI, sound and simulation code (prop/network managers, automata, window controls, `cSC4WinAnimationRecorder::Init` `0x00796F41`, which is a 2-argument flag setter); none is `cIGZDrawSystem::CreateBuffer` |
| The screen buffer (`span-render-4444.md` §1.2) | types 7 or 9 only, and it gets a SimGL context, not RenderToBuffer |
| Gimex loads, buffer resources | type from the caller (literals 2, 9, 1) or from the file header; a resource stored as type 4/6 would load as such before `ConvertBufferFromFormatToFormat` (`0x00931CF0`, 3 callers) converts to the factory target (3 or 1) |

**Result: no shipped path that draws spans into a type-4/6 RenderToBuffer target was found statically.** The
sweep cannot see types computed at run time (window `+0x14` pushes through memory operands fall outside the
16-byte window), so this is not a proof of absence. The deciding probe is a runtime counter at `0x0083078A`
(the 1555 vtable store) and at the five method entries during a normal session (SR1555-6).

**What a pixel test can use (explicitly constructed, not a game path).** Create a `cGZBuffer` through GZCOM
(CLSID `0xC470D325`), `Init` it with the pair (4 or 6, 16 bpp) (the renderer assumes 2 bytes per pixel and reads
the stride from the buffer), ask `cIGZDrawSystem` slot 4 for a context, then draw through the context's public
methods (SetAlphaBlend, SetGradientTransform, SetBltSource, DrawPreclippedSpanList via the clipping paths) and
read the buffer's pixels back. This exercises the shipped renderer bytes end to end but is a constructed
scenario: a report must label it "constructed buffer", never "in-game".

### 1.3 Driving the class through its native context (SR1555-7; re-span-context4) [exact, static]

span-render-4444.md §1.3 gives the full context ABI for types 4, 6 and 7, and the fixture route. For SR1555-7:
(a) After the constructor, `+0x24`, `+0x41` and `+0x74` are 0. Init (slot 3, `(buffer)`, pops 4, returns a byte)
sets `+0xA4` = 0. `+0x42`, `+0xB4` and `+0xC4` are set by neither and are indeterminate. (b) Setters (offset, pops): SetColorTable slot 7
`+0x1C` `(index, count, argb*)` 0xC, fills `+0xB4` (argb) and `+0xC4` (packed: types 4/6
`0x8000 | (r&0xF8)<<7 | (g&0xF8)<<2 | b>>3`); SetAlphaBlend slot 8 `+0x20` `(int)` 4; SetBltModulate slot 9 `+0x24`
`(byte)` 4; SetGradientTransform slot 12 `+0x30` `(m*, x0, y0)` 0xC; SetBltSource slot 13 `+0x34`
`(src, x, y, sx, sy, rect*)` 0x18; ClearBltTiling slot 14 `+0x38` - 0; SetBltTiling slot 15 `+0x3C` `(rect*)` 4.
(c) Shutdown (slot 4) unlocks with `0x8040`, the same flags as Init's lock, and only while `+0x75` is set. The
final Release does not unlock. (d) The textured path locks and unlocks the texture with `0x800` on both sides. The
bridge's "Unlock with the lock flags" matches the original.

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
**R**, `c3` is the alpha weight (§5 only).

**Producer** (`cGZDrawContextRenderToBuffer::SetGradientTransform`, `0x0082FDB0`, vtable `0x00AC17D0` slot 12;
`this`, `p` = pointer to 12 floats, `x0`, `y0` floats; `ret 0xC`). If `p` is null the gradient flag `+0x74` is
cleared and nothing else changes. Otherwise, for channel `i = 0…3` (record channel `i` = float index `i`, so
`p[0]` feeds B, `p[1]` G, `p[2]` R, `p[3]` alpha):

- `dx_i = R(p[4+i] × 16711680)`, `dy_i = R(p[8+i] × 16711680)`, where 16711680 = 255 × 65536 (double at
  `0x00AC17C8`) and `R(v)` = the product **rounded to float** (stored as a 32-bit float) and then converted with
  the FPU's current rounding mode (round-to-nearest-even by default: `fistp`);
- `s_i = R(p[i] × 16711680) + 0x8000` (start plus one half);
- `c_i = trunc(s_i − (dx_i × x0 + dy_i × y0))`, evaluated on the x87 stack (the integers loaded exactly, `x0`,
  `y0` as float, `+` before `−`) and truncated toward zero by `_ftol` (`0x009EEF04`).
- Then `+0x74 = 1`.

**x87 precision (SR1555-5b).** The function never writes the control word (no `FLDCW` in `0x0082FDB0`–`0x0082FE58`);
it runs under the calling thread's precision, which on the game's main thread is **24-bit** (`foundations.md`
§18.1). Under 24-bit precision every arithmetic result is rounded to a 24-bit significand, round-to-nearest-even:
the product `p × 16711680` (`FMUL` `0x0082FDD3`/`…DEB`/`…E03`; the following store to float is then exact),
`dx_i × x0` (`0x0082FE26`), `dy_i × y0` (`0x0082FE2C`), their sum (`FADDP` `0x0082FE30`) and `s_i − sum`
(`FSUBP`, bytes `DE E9` at `0x0082FE32`: the earlier-loaded `s_i` minus the sum). The `FILD` loads of the integers
`s_i`, `dx_i`, `dy_i` are exact. `_ftol` then truncates the 24-bit-rounded difference toward zero. Five roundings
per channel, no others. Worked examples (`p[i]`, `p[4+i]`, `p[8+i]`, `x0`, `y0` → `c_i`; `build-runs/gradprec.py`):

| Inputs | `s_i`, `dx_i`, `dy_i` | `c_i` at 24-bit | `c_i` if computed at 53-bit |
|---|---|---|---|
| 1.0, 0, 0, 0, 0 | 16744448, 0, 0 | 16744448 | same |
| 0.5, 0.01, 0.02, 100.25, 37.75 | 8388608, 167117, 334234 | −20982204 | same |
| 0.75, −0.003, 0.0007, 1023.3, 767.9 | 12566528, −50135, 11698 | **54886776** | 54886778 |
| 0.2, 0.1, 0.1, 333.3, 222.2 | 3375104, 1671168, 1671168 | **−924958720** | −924958694 |

An implementation that rounds each of the five results to float (IEEE single, nearest-even) and truncates the last
reproduces the 24-bit column (no intermediate here leaves the single exponent range for inputs in ±2³¹).

So the float inputs are channel fractions (1.0 → 255 in bits 16–23): the record is 8.16 fixed point with the
8-bit channel in bits 16–23, the start pre-rounded by +0.5, and `(x0, y0)` the pixel at which the colour equals
`p[0…3]`. Worked example: `p[0] = 1.0`, `p[4] = p[8] = 0`, `x0 = y0 = 0` → `c0 = 0x00FF8000`, `dx0 = dy0 = 0`;
RenderGradient then writes B = `(0x00FF8000 >> 16) & 31 = 31` (§4.2 takes the low 5 bits of the 8-bit value).
Neither `0x0082FDB0` nor the base `cGZDrawContext::SetGradientTransform` (`0x0082C360`, a forwarder through the
engine's vtable `+0x30`, itself only reachable through `cGZDrawContext` vtable slot `+0x6C`) has a direct
caller in the export; whether a shipped path reaches it is open (SR1555-2).

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
  `w` can exceed 32 only if `V_3` leaves 0…255.99 (bits ≥ 24); then the blend below is not a lerp but the
  packed form is still exact (`0x008313D9`–`0x00831421`): with `Sp = S & 0x7C1F`, `Dp = D & 0x7C1F`,
  `Sg = S & 0x03E0`, `Dg = D & 0x03E0`, `RB = ((Sp − Dp)·w + 32·Dp + 0x4010) mod 2³² >> 5`,
  `G = ((Sg − Dg)·w + 32·(Dg + 0x10)) mod 2³² >> 5`, output = low 16 bits of `(RB | 0x83E0) & (G | 0xFC1F)`.
  For `w ≤ 32` this equals the per-channel formula below. *[compat]*
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

### 6.1 How the caller fills the indices (`DrawPreclippedSpanList`, `0x008302E0`) [exact]

The context (`this`) calls the span renderer once per span list. With **no texture** (`+0x24` null):
gradient flag `+0x74` set → `RenderAlphaGradient` if alpha flag `+0x41` else `RenderGradient`, record `this+0x44`;
otherwise `RenderAlpha(…, src = this+0xC4, argb = this+0xB4)` if `+0x41` else `RenderSolid(…, this+0xC4)`. The
surface record is always `this+0x78`. (`+0x41` = `SetAlphaBlend(v)` stores `v > 0`, `0x0082FEA0`.)

With a texture it locks it (`vfunc +0x18`, flag `0x800`; failure → nothing drawn) and calls `RenderBlt(spans, n,
this+0x78, src, b, kind)`:

- `src` (24 bytes): `+0` pixel pointer (texture vfunc `+0x88`), `+4` stride (`+0x8C`), `+8` width (`+0x24`),
  `+0xC` height (`+0x28`), `+0x10` colour type (`+0x3C`), `+0x14` bytes per texel `bpp` (`+0x44`; the caller
  uses it only for the pointer bias below, no body reads it). The pointer and size are then adjusted per mode
  (§7.5, §7.6); in the plain (non-tiled, non-stretched) mode the pointer is biased by
  `−(ox × bpp + oy × stride)` (`0x00830554`), with `ox = ctx+0x28`, `oy = ctx+0x2C`, so destination (x, y)
  reads source texel `(x − ox, y − oy)`.
- `ox, oy` come from `SetBltSource` (`0x0082FFD0`, args `src, x, y, sx, sy, …` floats): it stores
  `ctx+0x30 = R(sx × 65536)`, `+0x34 = R(sy × 65536)`, `du = +0x38 = R(65536 / sx)`, `dv = +0x3C = R(65536 / sy)`
  (`R` as in §4.1: rounded to float, then `fistp`; 65536.0 at `0x00A8CEB0`). If `+0x30` and `+0x34` both equal
  `0x10000` it sets `ox = R(x)`, `oy = R(y)` (integers) and clears the stretch flag `+0x40`; otherwise
  `ox = R(du × x)`, `oy = R(dv × y)` (**16.16**, already scaled) and sets `+0x40 = 1`.
- `b.+0x10` **source-format index** from the texture colour type: 7 (A4444) → 0, 6 (A1R5G5B5) → 1, 4 (R5G5B5) → 2,
  5 (R5G6B5) → 3, 9 (A8R8G8B8) → 4, 8 (R8G8B8) → 5, 2 (8-bit) → 6; type 3 and any type outside 2…9 → **nothing
  drawn** (unlock and return).
- `b.+0x1C` **colour-key flag** (texture vfunc `+0x68` returns it and the key into `b.+0x18`), and the key is
  masked by source format: idx 0 `& 0x0FFF`, idx 1 and 2 `& 0x7FFF`, idx 3 `& 0xFFFF`, idx 4 and 5 `& 0xFFFFFF`;
  for idx 6 the flag is forced to 0, so row 13 is **unreachable**.
- `b.+0x14` = `this+0xB4` (the ARGB draw colour, used by the Modulate / ConstantAlpha bodies); `b.+0x00…+0x0C` the
  stretch or tiling parameters.
- `kind` = `alpha + 2 × modulate` (alpha = `+0x41`, modulate = `+0x42`, `SetBltModulate` `0x0082FEB0`), then
  `+8` if tiling is set (`+0xA4`, `SetBltTiling` `0x0082FE70`), else `+4` if stretching is set (`+0x40`); tiling
  wins over stretching.

## 7. The 1555 dispatch table (row = `2 × format + key`, column = `kind`)

Columns 0–11: `Blt`, `BltConstantAlpha`, `BltModulate`, `BltModulateAlpha`, the same four `Stretch`, the same four
`Tiled` (names from the Mac build; the column ↔ flag mapping of §6.1 is exact, the name per column is inferred
from that mapping). `*` = the slot points **outside** this range (identical-code folding with a sibling).

| Row | Source, key | Columns 0–11 (`0x008…`) |
|---|---|---|
| 0 | A4444, off | 31510 315B0 31680 317A0 31940 31A10 31B50 31CB0 31E80 31F50 320A0 32200 |
| 1 | A4444, on | 323D0 32490 32570 326B0 32870 32950 32AB0 32C20 32E00 32EF0 33070 331F0 |
| 2 | A1555, off | 48160* 333E0 33470 33590 33730 337C0 338D0 33A30 42C50* 33C00 33D20 33E80 |
| 3 | A1555, on | 34050 340D0 34170 342B0 34470 34510 34630 347A0 34980 34A30 34B70 34CF0 |
| 4 | X555, off | 34EE0 34F60 34FF0 35110 352A0 35330 35450 355A0 35760 35810 35940 35AA0 |
| 5 | X555, on | 35C70 35CF0 35D90 35ED0 36080 36130 36260 363C0 36590 36650 36790 36900 |
| 6 | R565, off | 36AF0 36B80 36C40 36D70 36F10 36FD0 37110 37270 37440 37510 37650 377C0 |
| 7 | R565, on | 379A0 37A50 37B30 37C80 37E40 37F10 38060 381D0 383C0 384C0 38630 387B0 |
| 8 | A8888, off | 389B0 38A60 38B30 38C20 38D90 38E60 38FB0 390D0 39270 39350 394C0 395F0 |
| 9 | A8888, on | 39790 39860 39960 39A70 39BF0 39CF0 39E60 39F90 3A140 3A240 3A3D0 3A510 |
| 10 | X888, off | 3A6D0 3A770 3A830 3A930 3AAB0 3AB80 3ACC0 3ADF0 3AF90 3B060 3B1B0 3B2E0 |
| 11 | X888, on | 3B490 3B550 3B640 3B750 3B8D0 3B9B0 3BB10 3BC50 3BE00 3BF00 3C070 3C1B0 |
| 12 | 8-bit, off | 3C370 3C3F0 3C500 3C5F0 3C760 3C800 3C930 3CA60 3CC00 3CCA0 3CDE0 3CF10 |
| 13 | 8-bit, on (unreachable) | 3D0B0 3D130 3D1D0 3D2E0 3D470 3D520 3D650 3D790 3D940 3DA00 3DB50 3DC90 |

### 7.1 Instantiation calling convention [exact for the bodies cited]

Each body is cdecl `(spans, n, dst, src, b)` with the three records by value (§6). Per span it walks the
destination pixels exactly as §1, and reads the source at `src.+0 + y × src.+4 + x × srcBytes` in the plain
column (`0x00834050`, `0x00848160`).

### 7.2 Verified bodies

- **Row 2 col 0, A1555 → 1555 `Blt`** (`0x00848160`, folded with a 4444 sibling): a raw 16-bit copy — every
  destination word = source word, **bit 15 included**.
- **Row 3 col 0, A1555 → 1555 keyed `Blt`** (`0x00834050`): per pixel, if `(src & 0x7FFF) == key` (the key
  already masked `& 0x7FFF`) the destination is left untouched, else it gets the raw source word (bit 15
  included). The key test ignores the source alpha bit.
- **Row 2 col 8** (`0x00842C50`) is folded with a 565 sibling: A1555 → 1555 tiled copy is a raw 16-bit copy
  with the §7.6 addressing (emulated, §7.7).

### 7.3 Source → 1555 conversion (column 0, `Blt`, key off) [exact for each cited body]

| Source | Body | Rule (`s` = source texel) |
|---|---|---|
| A4444 | `0x00831510` | each 4-bit channel `c4` → `c5 = (c4 << 1) \| (c4 >> 3)` (bit replication); A = bit 15 of `s` (alpha4 ≥ 8). Nibble order (SR1555-5a, from the masks at `0x00831566`–`0x00831594`): bits 0–3 → B, 4–7 → G, 8–11 → R, 12–15 alpha; each nibble lands at the top of its 5-bit field, then `+ ((w >> 4) & 0x0421)` copies each field's top bit into its low bit. Example: `0xF8C3` → `0xC726` |
| A1555 | `0x00848160` | raw copy, A copied |
| X555 | `0x00834EE0` | `(s & 0x7FFF) \| 0x8000` — **A forced to 1** |
| R565 | `0x00836AF0` | R = `s` bits 11–15, G = top 5 of the 6 green bits (bits 6–10), B = bits 0–4 (truncation, no rounding); A = 1 |
| A8888 | `0x008389B0` | each 8-bit channel → its top 5 bits (`>> 3`, truncation); A = bit 31 of `s` (alpha8 ≥ 128) |
| X888 | `0x0083A6D0` | as A8888, A = 1 |
| 8-bit | `0x0083C370` | the byte is an **alpha** (Mac `RGBFormat<255,0,0,0,1,0>`), not a palette index: output `0x7FFF \| ((byte & 0x80) << 8)` — white, A = byte ≥ 128 |

**Keyed rows (odd rows), every column.** Before anything else, per pixel: if `(s & mask) == key` the
destination pixel is left untouched (no blend, no write), else the key-off rule of the same column applies.
`s` is the **raw** source texel and `mask` the caller's mask of §6.1 (A4444 `0x0FFF`, A1555 and X555 `0x7FFF`,
R565 `0xFFFF`, A8888 and X888 `0xFFFFFF`), so the alpha bits never take part. The (unreachable) 8-bit keyed
row 13 uses an empty mask: it compares 0 with the key, so `key == 0` hides every pixel and any other key
draws every pixel as row 12. The key is `b.+0x18` as the caller left it (already masked).

### 7.4 Modulate, ConstantAlpha, ModulateAlpha [all 14 rows: §7.7]

- **Expand** (Modulate and ModulateAlpha only): the source texel is widened to 8-bit A, R, G, B
  (X555 `0x00834FF0` folds `+0x3FC0000` into the shift chain, then `+ ((x >> 5) & 0x070707)`):

| Source | A8 | R8, G8, B8 |
|---|---|---|
| A4444 | `a4 × 17` | each `c4 × 17` (`(c4 << 4) \| c4`) |
| A1555 | `0xFF` if bit 15 else `0x00` | each `(c5 << 3) \| (c5 >> 2)` |
| X555 | `0xFF` | each `(c5 << 3) \| (c5 >> 2)` |
| R565 | `0xFF` | R, B `(c5 << 3) \| (c5 >> 2)`; G `(g6 << 2) \| (g6 >> 4)` |
| A8888 | bits 24–31 | the bytes verbatim |
| X888 (4 bytes per texel) | `0xFF` | the bytes verbatim (bits 24–31 ignored) |
| 8-bit | the byte | `0xFF` each (white) |

- **Modulate** (column 2): each of the four 8-bit channels `c' = (c × m + 128) >> 8` with `m` the matching byte
  of `b.+0x14` (the ARGB draw colour; MMX `pmullw`/`paddw` with the constant `0x0080` words at `0x00AC1B80`,
  `psrlw 8`, `packuswb`; `(255·255 + 128) >> 8 = 254`, so the pack never saturates). The result is packed back by
  the A8888 rule of §7.3 (truncation; A = `c'_A ≥ 128`). No blend with the destination.
  The same for every source row (the expansion table above is the only per-row difference).
- **ConstantAlpha** (column 1, every row; e.g. X555 `0x00834F60`, A8888 `0x00838A60`): the source is converted
  as §7.3, then blended with the destination with weight `w = b.+0x14 >> 27` (top 5 bits of the draw colour's
  alpha, **0…31**, so never a full replace). The **source alpha is not used** by any row, A4444, A1555, A8888
  and 8-bit included: an A4444 texel with alpha 0 still blends its colour. The 14 column-1 bodies call the helper
  `0x0083C490` (its only 14 call sites); columns 3, 5, 7, 9, 11 inline the same blend; either way it is the §3 formula
  `C = floor((w·S + (32 − w)·D + 16) / 32)` per channel with **A = 1** (it ORs bit 20 before `>> 5`).
- **ModulateAlpha** (column 3, every row; e.g. X555 `0x00835110`): Modulate as above, then
  `w = c'_A >> 3` (top 5 bits of the modulated alpha, 0…31), blend as ConstantAlpha with `S` = the packed
  modulated colour, and A = 1. This is the only column family in which a source's own alpha weights the blend;
  for X555, R565 and X888 (A8 = `0xFF`) `w` = `((255·m_A + 128) >> 8) >> 3`, i.e. the draw colour's alpha.

Worked example (ConstantAlpha, X555 source `0x7C00` red over destination `0x001F` blue, colour alpha `0x80`):
`w = 0x80000000 >> 27 = 16`; R = `floor((16·31 + 16·0 + 16)/32) = 16`, B = `floor((0 + 16·31 + 16)/32) = 16`,
G = 0 → `0x8000 | 16<<10 | 16 = 0xC010`.

### 7.5 Stretch addressing (columns 4–7) [exact for X555 `0x008352A0`; other rows: §7.7]

`b.+0x00` = u0, `+0x04` = v0, `+0x08` = du, `+0x0C` = dv (16.16, used unsigned). For destination (x, y):
source row `= (v0 + dv × y) >> 16`, source column `= (u0 + du × x) >> 16`, then `u += du` per pixel —
nearest-texel, **no clamp, no wrap** (the caller must keep it in range). The texel is converted as §7.3.

**Caller values** (`0x008304EA`–`0x00830551`, taken when tiling is off and `ctx+0x40` is set): the source
pointer is **not** biased; `b.+0x00 = (du >> 1) − ox`, `b.+0x04 = (dv >> 1) − oy` (arithmetic shifts, 32-bit
wrap), `b.+0x08 = du`, `b.+0x0C = dv`, with `du, dv, ox, oy` from `SetBltSource` (§6.1, stretch case: `ox`,
`oy` already 16.16). So destination (x, y) samples source column `floor((du·x − R(du·x_s) + du/2) / 65536)`
(x_s the float blit x), i.e. the texel under the destination pixel's centre, all in unsigned 32-bit arithmetic.

### 7.6 Tiled addressing (columns 8–11) [exact for X555 `0x00835760`; other rows: §7.7]

**Caller** (`0x00830437`–`0x008304E7`, taken when `ctx+0xA4` is set, before the stretch test). With the tiling
rectangle `L = ctx+0x94`, `T = +0x98`, `Rt = +0x9C`, `B = +0xA0` (stored verbatim by `SetBltTiling`
`0x0082FE70`, which also sets `+0xA4 = 1`; `ClearBltTiling` `0x0082FE60` clears it):

- source pointer `+= L × bpp + T × stride` (top-left of the tile); `src.+8 = TW = Rt − L`,
  `src.+0xC = TH = B − T` (the tile replaces the texture size);
- `b.+0x00 = u0 = (M − (M mod TW) − ox) mod TW` and `b.+0x04 = v0 = (M − (M mod TH) − oy) mod TH`, with
  `M = 0x7FFFFFFF`, all unsigned 32-bit (`div`). For `0 ≤ ox ≤ M − (M mod TW)` this is `(−ox) mod TW`;
  `b.+0x08`/`+0x0C` are left unset (stale stack) and no tiled body reads them. `TW = 0` or `TH = 0` is a
  divide fault in the original *[compat: never pass an empty tile]*.

**Body.** Per span at (x0, y): source row `sy = (y + v0) mod TH`, source column `sx = (x0 + u0) mod TW`
(unsigned 32-bit `div`; x0 and y are the 16-bit span coordinates). Per pixel: read texel (sx, sy), convert and
store as the matching column 0–3 body, then `sx += 1`; when `sx` reaches TW it resets to 0 (the pointer goes back
to the start of the row). Net effect: destination (x, y) shows tile texel `((x − ox) mod TW, (y − oy) mod TH)`.

### 7.7 The 168 slots as one rule, and how it was checked [verified, all slots]

Every slot `(row, col)` computes, for each destination pixel of each span:
`texel` = addressing of `col / 4` (0 plain §6.1, 1 stretch §7.5, 2 tiled §7.6) read at the row's texel width
(2 bytes for rows 0–7, 4 for rows 8–11, 1 for rows 12–13); if the row is keyed and the key test of §7.3 hides
it, skip; otherwise `col % 4` = 0 conversion §7.3, 1 ConstantAlpha, 2 Modulate, 3 ModulateAlpha (§7.4).

**Check (round 2).** Each of the 168 slot addresses (the 166 bodies in the owner range plus the two folded
slots `0x00848160`, `0x00842C50`) was executed from the shipped bytes under a CPU emulator (unicorn 2.1.4,
`build-runs/emu1555.py`, committed with this spec, which maps `bin/SimCity4.exe` and calls the slot with the
records of §6) and its destination compared word for word with the rule above: **168/168 slots matched on
3000 random cases each (504 168 cases)**: random 16-bit destinations, texture sizes 4–40 × 1–6 with padded
strides, 1–4 spans, 25 % of texels forced to the key (bits outside the mask random), colours random plus
`0xFFFFFFFF`, `0x80808080`, `0x00FFFFFF`, `0xFF……`; stretch `du, dv` in `0x4000…0x20000`, tiled `u0 < TW`.
The five methods (`emu1555m.py`, §2–§6, RenderBlt checked to reach `rows[key + 2·fmt][kind]`) matched on 5000
cases each, AlphaGradient including `w > 32`. The 172nd function, `0x0083C490`, runs inside the
ConstantAlpha/ModulateAlpha slots that call it. Not covered by the check: `n = 0`, out-of-range stretch
coordinates (§7.5: no clamp), 32-bit wrap of `u0 + du·x`, tiled `u0 ≥ TW`, and the instruction-level CFG
(the check is behavioural: two slots that differ in register allocation but compute the same function pass
alike, which is what an implementation needs). A raw normalised-instruction diff (`build-runs/norm.py`) finds
no two slots byte-identical: rows differ in register use, so it gives no stronger evidence than the emulation.

## 8. Open points

Open: SR1555-2 (`docs/specs/questions/span-render-1555.md`): whether any shipped path draws a gradient into a
1555 buffer. Narrowed (re-span1555-bridge4): `0x0082FDB0` and `0x0082C360` have no direct caller; the only
reference to `0x0082C360` is its `cGZDrawContext` vtable entry `0x00AC1734` (`+0x6C`), and the image has 594
`CALL [reg+0x6C]` sites whose receiver type is not static, so it needs the same runtime probe as SR1555-6, which
must hit first (no 1555 target, no 1555 gradient). SR1555-6: whether any shipped path creates a type-4/6
RenderToBuffer target (§1.2). SR1555-1, -3, -4, -5 are answered (§4.1, §6.1, §7.3–§7.7). Every function of the owner range is
[exact] or [verified]: 5 methods, 166 in-range slot bodies, the blend helper `0x0083C490`.
