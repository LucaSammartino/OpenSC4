# Span renderer A4R4G4B4 (`nGZDrawSoftRender::cSpanRender4444`)

Status: RE spec, lane re-span4444-4, 2026-10-21. Source: SimCity 4 Deluxe GOG 1.1.641 Windows image, Ghidra export
(`re_query.py asm`) and raw table reads from the image. Every numeric rule below was read from the instructions at the
cited addresses; nothing is inferred from "usual" RGBA maths. Round 2 (same lane, same day) verified all 168 blit
kernels by executing their bytes against the model (section 9.5).

Scope: the owner range `0x00847CE0`-`0x00854E10` (172 functions, 51,809 bytes, `tools/coverage-ranges.tsv:21`): the
five vtable methods, the 166 blit kernels of its `RenderBlt` table that live in the range, and one helper.
Out of scope (other owners): `cGZDrawContextRenderToBuffer` itself (view3d.md section 7), the 1555/565/8888/8 span
renderers, fonts, base widgets, the DX7 driver.

## 1. Class and creation

- vtable `0x00AC185C`, 6 slots: 0 deleting destructor `0x0082FF00` (shared by all five span renderers; resets the
  vtable to `cISpanRender` `0x00AC17AC`, frees when flag bit 0 is set), 1 `RenderSolid` `0x00847CE0`,
  2 `RenderGradient` `0x00847D30`, 3 `RenderAlpha` `0x00847E40`, 4 `RenderAlphaGradient` `0x00847F10`,
  5 `RenderBlt` `0x008480C0` (Mac slot order, `re/symbols/gzdraw-softrender.tsv`).
- Created by `cGZDrawContextRenderToBuffer::Init` (`0x00830710`) when the target buffer's colour type is 7
  (A4R4G4B4), stored at context `+0xB0`. The object has no data members the methods read: every method takes all
  its inputs as arguments (`this` is never dereferenced in slots 1-5).

### 1.1 Native call boundary (SP3, re-spanbridge4444-4)

All facts below were read from the instruction bytes at the cited addresses.

**Object and vtable.** The renderer object is 4 bytes: only the vtable pointer. `Init` allocates it per context
with `GlobalHeapAlloc(4)` (`0x005E55E0`, call at `0x008307AA`), stores `0x00AC185C` into it (`0x008307B6`; jump
table `0x008308B0` indexed by colour type - 2, entry 5 = type 7) and the pointer into context `+0xB0`
(`0x008307EE`); a failed allocation stores 0. The vtable is one static table in `.rdata`: `0x00AC185C` occurs once
in the whole image as an immediate (that store), and each slot address 1-5 occurs once in the image (its own
entry; slot 0 `0x0082FF00` is shared with the other five span vtables and `0x00AC17AC`). So every A4R4G4B4 context points at the same six entries: patching entries 1-5 of `0x00AC185C` covers all
instances; nothing copies the table.

**Lifetime.** Context `Shutdown` (`0x0082FF20`, slot 4 of `0x00AC17D0`, and the first thing `Init` calls) and
`Release` (`0x008306C0`) call slot 0 with flags 1 and clear `+0xB0`. Slot 0 (`0x0082FF00`) is thiscall with one
dword (`RET 4`): it sets the vtable to `0x00AC17AC` and, when flag bit 0 is set, frees with `GlobalHeapFree`
(`0x005E5620`). A substitute object placed in `+0xB0` must therefore survive a slot-0 call with flags 1 that
releases it through its own slot 0.

**Calling convention (slots 1-5).** MSVC thiscall: `this` in ECX (ignored by all five; ECX is overwritten before
any read), arguments pushed right to left, the callee pops them, nothing is returned (EAX is undefined and no call
site reads it). EBX, EBP, ESI and EDI are preserved (each slot pushes and pops the ones it uses); the caller relies on
ESI after the call.

| Slot | Offset | Method | Arguments (dwords, in order) | Callee pops |
|---|---|---|---|---|
| 0 | +0x00 | deleting destructor `0x0082FF00` | `flags` (1) | `RET 4` |
| 1 | +0x04 | RenderSolid `0x00847CE0` | `spans, count, buffer, colour` (4) | `RET 0x10` (`0x00847D23`) |
| 2 | +0x08 | RenderGradient `0x00847D30` | `spans, count, buffer, grad` (4) | `RET 0x10` (`0x00847E33`) |
| 3 | +0x0C | RenderAlpha `0x00847E40` | `spans, count, buffer, colour, alpha` (5) | `RET 0x14` (`0x00847F00`) |
| 4 | +0x10 | RenderAlphaGradient `0x00847F10` | `spans, count, buffer, grad` (4) | `RET 0x10` (`0x008480B3`) |
| 5 | +0x14 | RenderBlt `0x008480C0` | `spans, count, rectA, rectB, src, mode` (6) | `RET 0x18` (`0x0084814F`) |

`buffer`, `grad`, `rectA`, `rectB` and `src` are pointers; `colour`, `alpha`, `count` and `mode` are by value.
The kernels RenderBlt calls are cdecl with 22 dwords (`spans`, `count`, 6 + 6 + 8 dwords of the copied records),
popped by RenderBlt (`ADD ESP,0x58` at `0x0084814A`).

**The only native caller** is `cGZDrawContextRenderToBuffer::DrawPreclippedSpanList` (`0x008302E0`, thiscall
`(spans, count)`, `RET 8`; slot 22 of `0x00AC17D0`). Byte sweep of `0x0082FD60`-`0x008310B0` for a `+0xB0`
displacement finds only the constructor, `Init`, `Shutdown`, `Release` and the four loads in this function (`0x008304AD`, `0x00830529`, `0x00830583`, `0x008305C2`); the
slots of `0x00AC17D0` outside that range are shared 1-8 byte stubs that do not touch `+0xB0`. The function picks
one slot per call from context fields:

| Context byte `+0x74` (gradient) | Context byte `+0x41` (alpha) | Call | Extra arguments from the context |
|---|---|---|---|
| 0 | 0 | slot 1 (`0x00830647`) | `buffer = ctx+0x78`, `colour = [ctx+0xC4]` |
| 0 | non-zero | slot 3 (`0x00830627`) | `buffer = ctx+0x78`, `colour = [ctx+0xC4]`, `alpha = [ctx+0xB4]` |
| non-zero | 0 | slot 2 (`0x008305FC`) | `buffer = ctx+0x78`, `grad = ctx+0x44` (12 dwords) |
| non-zero | non-zero | slot 4 (`0x008305E7`) | `buffer = ctx+0x78`, `grad = ctx+0x44` |

The textured path (three sites `0x008304D1`, `0x0083053B`, `0x00830591`) calls slot 5 with `rectA = ctx+0x78`,
`rectB` and `src` in locals of the caller, and `mode` from EDI (one site ORs 4 into it at `0x0083050F`). `spans`
and `count` are always the caller's own two arguments, passed through unchanged.

**No interface reaches the renderer.** No slot of `cIGZDrawContext` (`0x00AC17D0`) returns or sets `+0xB0`.
`cGZDrawContextRenderToBuffer` is created only by the factory at `0x0082B910` (the image's only `E8` call to the
constructor `0x00830690`): slot 3 of a 4-slot factory vtable `0x00AC162C` (QueryInterface `0x0082B850` accepts
IIDs 1 and `0x6B2EA76C`), thiscall `(buffer, out)`, `RET 8`, returns a bool in AL. It queries the buffer for
`0x20732180`, allocates 0xD4 bytes, constructs, AddRefs into `*out`; it does not call `Init`.
`cGZDrawSystem::Init` (`0x0082BC90`) appends this factory to the draw system's factory list, and
`cIGZDrawSystem` vtable `0x00AC1668` slot 4 (`0x0082BAB0`, thiscall `(buffer, out)`, `RET 8`) tries each factory
in list order until one returns true. `kcGZDrawContextAsBuffer` `0xAB300B20` is not on this path: its only code
uses are the unreferenced 6-byte getter `0x0082ABD0` and `cGZScreenBuffer::Init` (`0x00992044`), which passes it
with `0xAB300B2B` to slot `+0xC` of the GZCOM object. So a plugin can reach a 4444 renderer only by patching the
shared vtable entries, or by writing context `+0xB0` after `Init` (no interface does that).

**Which shipped path creates a colour-type 7 target: not found statically** (open as SP4). None of the 34 direct
calls to the colour-pair constructor `0x0044A490` pushes the literal 7 (literals seen: 1, 3, 9; the rest take the
type from data: buffer resources, `cGZGimexFactory::LoadWithReader`, `cSLBuffer::Load`), and the ten places in the image where
`PUSH 7` precedes a `+0x14` virtual call within 16 bytes are all simulation code (prop and network managers,
automata, vehicle collision), not calls of `cIGZDrawSystem` slot 5 `CreateBuffer(out, width, height, colourType)`
(`0x0082B980`, `RET 0x10`).

### 1.2 Where colour type 7 comes from in the shipped game (SP4, re-spanbridge4444-4 round 2)

Static result, read from instruction bytes. No runtime probe was run.

**The one shipped producer: the screen buffer.** `cGZGraphicSystemW32::OnCanvasInit` (`0x0098B2A0`) re-initialises
the graphic system's screen buffer (`cGZScreenBuffer`, object `+0x100`, created by `0x00991DA0`) when the system's
byte `+0x41` is set. The constructor sets that byte to 1 (`0x0098FDC0`). The colour type is chosen at
`0x0098B3CC`-`0x0098B3DD`:

| Screen bit depth (`d`) | Colour type passed |
|---|---|
| `d <= 16` | 7 (A4R4G4B4, 16 bpp) |
| `d > 16` | 9 (A8R8G8B8, 32 bpp) |

`d` is the dword at `+0xC` of the record that GDriver slot `0x100` fills (`0x0098B37F`, `0x0098B391`). If no
GDriver is present, `d` = 32 (`0x0098B363`). Width and height are the global screen settings `0x00B17AE4` /
`0x00B17AE8`. The call goes through the screen buffer's interface at `+0x4C` (vtable `0x00ADB388`, slot 4 =
`cGZScreenBuffer::Init` `0x00991EC0`). Init stores the type and bpp unchanged in the buffer's colour-type pair (base
`+0xC`/`+0x10`, `0x00991F79`-`0x00991F7E`). It also creates a 4 x 4 auxiliary `cGZBuffer` of the same colour type
(`+0x30`; `0x00991FA5`-`0x00991FAE`) and fills it with one colour. So in a 16-bit screen mode, both the screen buffer
and its 4 x 4 buffer have colour type 7.

**That buffer never gets an A4R4G4B4 span renderer.** `cIGZDrawSystem` slot 4 (`0x0082BAB0`) gives a buffer a draw
context by trying the registered context factories in list order: the first one whose slot 3 succeeds wins. Slot 3
(`0x0082BD30`) puts each new factory at the front of the list (`0x0082BD6B`-`0x0082BD70`). `cGZScreenBuffer::Init`
registers a `cGZDrawContextRenderToSimGLFactory` (`0x00991E50`, call at `0x00992021`). That factory comes after the
`cGZDrawContextRenderToBuffer` factory that `cGZDrawSystem::Init` registered, so it sits in front of it. Its create
method (`0x00993270`) accepts only buffers that answer IID `0xE3094DC8`, and `cGZScreenBuffer::QueryInterface`
(`0x009917B0`) answers that IID (`0x009917B4`; nothing else in the image tests it). The screen buffer therefore gets a
SimGL context, not `cGZDrawContextRenderToBuffer`. Nothing in `Init` asks for a draw context for the 4 x 4 buffer.

**Other sources checked, none of which yields type 7 for a RenderToBuffer target:**

| Source | Colour type | Note |
|---|---|---|
| `cGZBuffer` (CLSID `0xC470D325`, creator `0x008298F0`, constructor `0x008272E0`) | default 3 / 16 bpp | `Init` (`0x008269B0`) stores the caller's pair unchanged; `GetColorType` is slot `0x3C` |
| `cGZWin::create_private_buffer` -> `cIGZDrawSystem` slot 5 (`0x0082B980`) | 1 | stays 1 (bpp 0, 0-byte pixel store `0x00826A50`). The context made there is `cGZDrawContext` (`0x0082C9A0`, vtable `0x00AC16C8`), and `RenderToBuffer::Init` would reject type 1 anyway (default case `0x0083087A` releases the buffer and returns 0 for any type outside 2..9) |
| Gimex image loads (`cIGZGimexFactory` IID `0x02FFBB60`; `LoadFromFile` slot 4 arg 3 = target type) | caller's choice | literal targets found: 2 (`0x007428A3`), 9 (`0x007BFDB4`), 1 (`0x008FE752`). `TranslateGimexToGZBuffer` (`0x0090C1D4`) has a type-7 case (`TranslateGimex24ToGZBufferARGB4444Dither` `0x0090BF3C`), but no caller with target 7 was found |
| Buffer resources v1/v2 (persistence.md 6.2) | from the file header | load as the file's type, then `ConvertBufferFromFormatToFormat` (`0x00931CF0`) to the target. Factory defaults: `cGZBufferResourceFactory` 3 (`0x0097DBD2`), `cGZPersistBufferResourceFactory` 1 (`0x0097807C`). `SetColorType` (`0x0097CF60`, `0x00978097`) has no direct callers |
| Lot-configuration editor texture-to-buffer conversion (`0x00604920`) | 8 or 9 | type 3 maps to 9, anything else to 8 (`0x006049A2`-`0x006049AD`) |

**What a game-pixel test can use.** A type-7 RenderToBuffer target can be made only through `cIGZDrawSystem` slot 4.
It needs a `cIGZBuffer` that has been `Init`-ed with colour type 7 (for example a `cGZBuffer` made through GZCOM CLSID
`0xC470D325`) and that does not answer `0xE3094DC8`. The `cGZDrawContextRenderToBuffer` factory (`0x0082B910`) accepts
any object that answers `cIGZBuffer` `0x20732180` (`0x0082B91C`). None of these interfaces is in the public SDK
headers, so this route is undocumented. It works by calling the game's own COM classes and is not a game path. The
static evidence finds no shipped scenario in which the game itself sends A4R4G4B4 spans to `0x00AC185C`.

### 1.3 The buffer draw context as a native fixture route (SP5, SR1555-7; re-span-context4) [exact, static]

This section is shared by the 1555 (types 4, 6), 565 (5), 4444 (7) and 8888 (8, 9) renderers: one context class,
`cGZDrawContextRenderToBuffer`, picks the renderer in `Init`. Every slot, pop count and constant below was read from
the shipped bytes (raw vtable dwords, `RET` immediates); REA had no binary open, so this is static evidence only.

**Getting the objects.** (1) Buffer: GZCOM class `0xC470D325`, interface `cIGZBuffer` `0x20732180` (`cGZBuffer`
QueryInterface answers only 1, `0x20732180` and `0x86D72B57`). (2) Draw system: framework slot `+0x14`
GetSystemService called as `(0xEB2EA3CB, 0x4B2EA3B3, &out)` at `0x0082C8D1`/`0x0082C8CC`/`0x0082C8D8`: the **service
ID is `0xEB2EA3CB`** and the **interface ID is `0x4B2EA3B3`** (`cGZDrawSystem` QueryInterface tests `0x4B2EA3B3`,
`0x0082B894`). The template name `cRZSysServPtr<cIGZDrawSystem, 0x4B2EA3B3, 0xEB2EA3CB>` lists IID first, then
service ID. (3) Context: draw system slot 4 (`0x0082BAB0`, entry `0x00AC1678` of vtable `0x00AC1668`), thiscall
`(object, &out)`, RET 8, returns a byte. It asks each factory of the list at `+0x20` in order and stops at the first
one that returns non-zero. The RenderToBuffer factory (vtable `0x00AC162C`, slot 3 `0x0082B910`) is added by the draw
system's own `Init` (`0x0082BC90`, appended at the tail). It accepts any object that answers `0x20732180`, allocates
0xD4 bytes, runs the constructor `0x00830690`, stores the pointer in `*out` **without releasing what `*out` held**,
and AddRefs it (count 0 to 1). The caller owns that one reference. The SimGL factory (`0x00993270`) accepts only
`0xE3094DC8`, which `cGZBuffer` does not answer. A fixture must still check that the vtable pointer of `*out` is
`0x00AC17D0` before it uses the offsets below.

**cIGZBuffer slots used** (vtable `0x00AC1400`, all thiscall):

| Slot (offset) | Entry target | Call | Pops | Returns |
|---|---|---|---|---|
| 3 (`+0x0C`) | `0x008269B0` Init | `(width, height, type, bits)`, four dwords by value | 0x10 | byte: 0 if already initialised, width 0 or height 0, or allocation failed; else 1 |
| 6 (`+0x18`) | `0x00826AA0` Lock | `(flags)` | 4 | byte: 0 only if the pixel pointer `+0x3C` is null |
| 7 (`+0x1C`) | `0x00826490` Unlock | `(flags)` | 4 | byte, always 1 |
| 30 (`+0x78`) | `0x00825DA0` pack colour | `(r, g, b)`, low byte of each dword | 0xC | packed pixel (below) |
| 34 (`+0x88`) | `0x008265C0` | `()` | 0 | pixel (0, 0) pointer (`+0x3C`) |
| 35 (`+0x8C`) | `0x0068D1B0` | `()` | 0 | row stride in bytes (`+0x40`) |

Init stores width `+0x1C`, height `+0x20`, type `+0x0C`, bits `+0x10`, then allocates (`+0xAC`, `0x00826A50`): stride
= `(width × (bits >> 3) + 3) & ~3`, pixels = stride × height bytes (contents not cleared there). Examples: 32×8,
bits 16 → stride 64; 16×16, bits 16 → 32; 9 wide, bits 32 → 36; 3 wide, bits 16 → 8. So a plug-in reads pixels as:
Lock(f), base = slot 34, stride = slot 35, ..., Unlock(f).

**Lock and Unlock, audited.** Lock ORs `flags` into the word `+0x44`, increments the 16-bit count `+0x38`, and
increments `+0x30` when bit 15 of `flags` is set. Unlock clears the bits of `flags` from `+0x44`, decrements `+0x38`
and does not check the flags, the count, or `+0x30`, and does not decrement `+0x30`. Unlock with different flags
therefore leaves bits set. Every pair in the context uses the same flags on both sides:

| Lock site | Flags | Matching unlock site(s) | Flags |
|---|---|---|---|
| context Init `0x00830750` (destination) | `0x8040` | Shutdown `0x0082FF20` (only while `+0x75` is set), Suspend `0x008302B4` | `0x8040` |
| Resume `0x008302D4` (destination, only while `+0x75` is clear) | `0x8040` | as above | `0x8040` |
| DrawPreclippedSpanList `0x008302FB` (texture `+0x24`) | `0x800` | `0x008304DE`, `0x00830548`, `0x0083059E` (after the renderer), `0x008305AA` path (no renderer) | `0x800` |

So "Unlock with the flags it locked with" **matches** the original for every context path. The context's last
`Release` (`0x008306C0`) deletes the renderer, releases `+0x24` and `+0x8`, frees itself and **does not unlock**. A
fixture must call Shutdown (slot 4) before its last Release, or the buffer keeps `0x8040` and one count.

**Context vtable `0x00AC17D0`** (all thiscall; "-" = no stack arguments):

| Slot | Target | Name in the export | Arguments | Pops | Effect on the fields `DrawPreclippedSpanList` reads |
|---|---|---|---|---|---|
| 0 | `0x0082FD60` | QueryInterface | `(iid, out*)` | 8 | answers 1 and `0x2AE78A07` only |
| 1 | `0x005F47F0` | AddRef | - | 0 | `+4` += 1 |
| 2 | `0x008306C0` | Release | - | 0 | see above |
| 3 | `0x00830710` | Init | `(buffer)` | 4 | byte; see span-render-565.md §1.1. Sets `+0xB0`, `+0x78`..`+0x90`, `+0xA4` = 0, `+0x75` = 1 |
| 4 | `0x0082FF20` | Shutdown | - | 0 | deletes renderer, `+0xB0` = 0; if `+0x75`: Unlock(`0x8040`); releases `+8` |
| 5 | `0x00A64430` | (forwarder) | passed through | callee's | jumps to `cIGZBuffer` `+0x2C` of `+8` |
| 6 | `0x0082FD90` | SetClipRect | `(rect*)` 16 bytes | 4 | `+0x14`..`+0x20` (not read by slot 22) |
| 7 | `0x0082FF70` | SetColorTable | `(index, count, argb*)` | 0xC | for each i: `+0xB4+4(index+i)` = argb, `+0xC4+4(index+i)` = pack(argb>>16, argb>>8, argb). No bounds check (room for 4) |
| 8 | `0x0082FEA0` | SetAlphaBlend | `(int)` | 4 | `+0x41` = (value > 0), signed (`SETG`) |
| 9 | `0x0082FEB0` | SetBltModulate | `(byte)` | 4 | `+0x42` = low byte |
| 10 | `0x0082FEC0` | SetAntialiasing | `(byte)` | 4 | `+0x43` (read by slot 21 only) |
| 11 | `0x00648F10` | (shared no-op) | `(x)` | 4 | none |
| 12 | `0x0082FDB0` | SetGradientTransform | `(m*, x0, y0)`, x0/y0 floats | 0xC | null m: `+0x74` = 0. Else `+0x44+4i`, `+0x54+4i`, `+0x64+4i` from the 12 floats of m (span-render-1555.md §4.1), `+0x74` = 1 |
| 13 | `0x0082FFD0` | SetBltSource | `(src, x, y, sx, sy, outRect*)`, x..sy floats | 0x18 | null src: releases `+0x24`, returns 0. Else releases `+0x24`, queries src for `0x20732180` into `+0x24`; if that fails returns **1** with `+0x24` = 0. Else sets `+0x28`..`+0x3C`, `+0x40` (0 when both scales give `0x10000`, else 1), `+0xA8`/`+0xAC`, writes the 16-byte destination rect (section 1.3.1), returns 1 |
| 14 | `0x0082FE60` | ClearBltTiling | - | 0 | `+0xA4` = 0 |
| 15 | `0x0082FE70` | SetBltTiling | `(rect*)` 16 bytes | 4 | `+0xA4` = 1, `+0x94`..`+0xA0` |
| 16, 20 | `0x00407990` | (bare `RET`) | - | 0 | none |
| 17 | `0x008302A0` | Suspend | - | 0 | if `+0x75`: Unlock(`0x8040`), `+0x75` = 0 |
| 18 | `0x008302C0` | Resume | - | 0 | if not `+0x75`: Lock(`0x8040`) (result not tested), `+0x75` = 1 |
| 19 | `0x0082FED0` | LockTarget | `(record*, x)` (second dword not read) | 8 | copies `+0x78`..`+0x8C` (base, stride, w, h, type, bytes per pixel) to record, returns 1; no lock |
| 21 | `0x00831000` | Draw | `(prim, verts*, count)` | 0xC | antialiased polygons only when `+0x43` is set |
| 22 | `0x008302E0` | DrawPreclippedSpanList | `(spans, n)` | 8 | one renderer call (span-render-565.md §1.1 path table) |
| 23 | `0x0082FF00` | (this dword is slot 0 of the 1555 span table `0x00AC182C`) | | | |

**State after the constructor and Init.** The constructor sets the vtable, `+4` (count), `+8`, `+0x24`, `+0x41`,
`+0x43`, `+0x74`, `+0x75`, `+0xB0` to 0. Init then sets the fields listed above. It does **not** set `+0x40`, `+0x42`,
`+0x44`..`+0x73`, `+0xB4`..`+0xD0` or the texture fields. They hold whatever the 0xD4-byte heap block held. A fixture
must set every field its chosen path reads before slot 22 is called:

| Path (span-render-565.md §1.1) | Set first |
|---|---|
| solid (slot 1) | slot 7 `(0, 1, &argb)`, slot 8 `(0)`, slot 12 `(null, 0, 0)` |
| alpha (slot 3) | slot 7, slot 8 `(1)`, slot 12 `(null, ...)` |
| gradient (slot 2) / alpha gradient (slot 4) | slot 12 `(m, x0, y0)`, slot 8 `(0)` or `(1)` |
| texture (slot 5) | slot 13 (sets `+0x24`, `+0x40`), slot 9, slot 8, slot 14 or 15 |

Pack (`0x00825DA0`, table `0x00825E60`, by type `+0x0C`): types 4 and 6 →
`0x8000 | (r & 0xF8) << 7 | (g & 0xF8) << 2 | b >> 3`. Type 5 → `(r & 0xF8) << 8 | (g & 0xFC) << 3 | b >> 3`. Type 7 →
`0xF000 | (r >> 4) << 8 | (g >> 4) << 4 | b >> 4`. Types 8 and 9 → `0xFF000000 | r << 16 | g << 8 | b`. Any other
type (including 2) → 0. The alpha byte of argb is not passed. Example: argb `0x80FF8040` packs to `0xFE08` (types 4/6),
`0xFC08` (5), `0xFF84` (7), `0xFFFF8040` (8/9). `+0xB4` keeps `0x80FF8040`.

**Fixture route (original versus ours, through the native context).**

1. Create the buffer and call Init `(w, h, type, 16)` for type 4, 6 or 7. Fill its pixels after Lock(`0x8040`) and
   Unlock(`0x8040`).
2. Get the draw system (service `0xEB2EA3CB`, IID `0x4B2EA3B3`). Call slot 4 `(buffer, &ctx)` with `ctx` = null.
   Check that the result is non-zero and that the vtable is `0x00AC17D0`. Release the draw system.
3. ctx slot 3 `Init(buffer)`. Expect 1 and `+0xB0` holding vtable `0x00AC182C` (types 4/6) or `0x00AC185C` (7).
   Then set the state of the path under test (table above).
4. Snapshot the destination. Call slot 22 `(spans, n)` with the original span-table entries in place, and keep the
   result. Restore, patch the entries to ours, call again, and compare. Byte-exact for solid, alpha and blit.
   Gradients need span-render-1555.md §4.1 (precision).
5. ctx slot 4 Shutdown, ctx slot 2 Release, buffer Release. After Shutdown, `+0x38` of the buffer is back to its
   value before step 3.

Counting and labels: count renderer calls by the **exact return address** in section 1.3.2. A call from one of the seven sites of
`0x008302E0` while the fixture's slot-22 call is active is **"constructed (native context)"**. A call from those
sites at any other time is **"shipped"**. A direct call from the fixture to a slot entry is **"constructed
(direct)"**. Report the three counts separately. Only "shipped" counts are evidence for SP4, SR1555-6 or SR565-5.

### 1.3.1 Source placement, scales and rectangle directions (SP6a) [exact, static]

These are the **framework 2D** context's contracts. They agree with the shared texture-state rules in
[span-render-1555.md §6.1](span-render-1555.md#61-how-the-caller-fills-the-indices-drawpreclippedspanlist-0x008302e0-exact).
They apply to 4444 because `Init` installs `0x00AC185C` in `+0xB0` for colour type 7
(`0x008307A8`–`0x008307BC`), while the setters and `DrawPreclippedSpanList` remain the same functions.
[draw-context.md](draw-context.md) describes `SC4DrawContext`, the separate **3D** object; its texture/render-state
contracts do not specify these setters.

**SetBltSource.** The six arguments are `(src, x, y, sx, sy, outRect*)`, with four binary32 floats and a writable
16-byte output rectangle. `x,y` place the source in **destination pixel coordinates**. `sx,sy` are destination
size per source texel: 2 doubles the displayed size, 0.5 halves it. They are not source offsets or the sampling
steps themselves. The source is queried for `cIGZBuffer`; the setter obtains its bounds through buffer slot 11
(`+0x2C`, call `0x00830017`). For `cGZBuffer` that slot is `0x008268D0` (vtable entry `0x00AC142C`), which copies
the four signed edge coordinates `(L,T,R,B)`. A newly Init-ed buffer has `(0,0,width,height)`
(`0x008269CC`–`0x008269DD`). The caller's `outRect` contents are not read on success: it is **not a crop request**.

The stored values and mode selection are:

| Context fields | Meaning | Native conversion |
|---|---|---|
| `+0x30`, `+0x34` | forward scale in 16.16 | `R(sx*65536)`, `R(sy*65536)` |
| `+0x38`, `+0x3C` | source steps `du,dv` in 16.16 | `R(65536/sx)`, `R(65536/sy)` |
| `+0x40 = 0`, `+0x28`, `+0x2C` | plain mode, integer placement `ox,oy` | when **both stored forward scales** equal `0x10000`: `ox=R(x)`, `oy=R(y)` |
| `+0x40 = 1`, `+0x28`, `+0x2C` | stretch mode, scaled placement in 16.16 | otherwise: `ox=R(du*x)`, `oy=R(dv*y)` |

Here `R` means arithmetic under the caller's x87 precision, a store/reload as binary32, then signed dword integer
conversion using the current x87 rounding mode. The plain `R(x),R(y)` needs only the integer conversion of the
already-binary32 argument. There is no control-word change in this setter. Under the game's 24-bit,
nearest-even mode (shared precision rule in span-render-1555.md §4.1), retain the float rounding of each multiply
or divide before conversion. The constant `65536.0` is a **double** at `0x00A8CEB0`. Anchors:
forward/reciprocal conversions `0x0083001A`–`0x00830082`; quantized-scale test and plain placement
`0x00830085`–`0x008300B3`; scaled placement `0x008300F6`–`0x0083012E`.

The destination output rectangle is `(left,top,right,bottom)`, with exclusive right/bottom edges:

- Plain: copy `(L,T,R,B)` and add `ox` to **both** horizontal edges, `oy` to **both** vertical edges
  (`0x008300B7`–`0x008300F1`, final bottom store `0x00830259`).
- Stretch: the four edges come from `x`, `y`, `x+(R-L)*sx`, `y+(B-T)*sy`, respectively
  (`0x00830121`–`0x00830259`). The edge conversion subtracts double `0.5` (`0x00A92D28`), uses float `0.5`
  (`0x00A84D2C`) and `12582912.0` (`0x00AC17A0`) to round, and corrects exact half ties down. For the finite,
  small pixel coordinates below, each edge is `ceil(t-0.5)` in nearest-even mode. Products and sums still follow
  the caller's x87 precision; this is not permission to evaluate an entire edge expression in double.
- `+0xA8 = right-left`, `+0xAC = bottom-top`, using wrapping dword subtraction
  (`0x00830256`–`0x0083026A`). Width/height are derived values, not the last two rect members.

The draw consumer proves the direction of the mapping: plain mode biases the source pointer by
`-(ox*bytesPerTexel + oy*stride)` (`0x00830554`–`0x0083056A`), so destination `(X,Y)` samples source
`(X-ox,Y-oy)`. Stretch builds `u0=(du arithmetic-right-shift 1)-ox`, `v0=(dv arithmetic-right-shift 1)-oy`
(`0x008304EF`–`0x00830538`); its 16.16 walk is the section 9.1 mapping, including the half-pixel centre.

**SetBltTiling.** Slot 15 reads a **source texel rectangle** `(left,top,right,bottom)` into `+0x94`..`+0xA0` and
sets `+0xA4=1` (`0x0082FE70`–`0x0082FE97`). It does not write back to that rectangle. Draw subtracts left from
right and top from bottom to get the tile's source dimensions, then biases the source pointer by
`left*bytesPerTexel + top*stride` (`0x0083043D`–`0x00830469`); it passes those reduced dimensions to the tiled
renderer. Thus this rect is neither `(x,y,width,height)` nor a destination drawing extent. Tiling wins over
stretching (`0x00830431`–`0x00830437`); tile edges are not multiplied by `sx/sy`. Use ClearBltTiling to restore
plain/stretch selection. For the ordinary positive dimensions/placement in the examples, tile offsets are
`u0=(-ox) mod tileWidth`, `v0=(-oy) mod tileHeight` with nonnegative remainders (the native unsigned division
sequence is `0x0083046D`–`0x008304B3`; general wrapped offsets remain as specified in section 9.1).

**Caller obligations.** Successful source setup dereferences `outRect`; tiling dereferences its input rect.
Neither accepts a null rect on that path, validates scales, clips a source rectangle, or checks resulting
sampling bounds. A normal fixture must supply finite nonzero scales, nonempty source/tile dimensions and spans
whose sampled texels are inside the supplied allocation. A zero tile dimension reaches division by zero in the
draw context before any renderer call. Null `src` instead clears/releases the old source and returns 0 without
touching `outRect` (`0x0083027B`–`0x00830298`). Non-null `src` whose interface query fails returns **1**, leaves
`+0x24=0`, and leaves `outRect` and the other texture state untouched (`0x00830004`–`0x00830008` branch to
`0x00830270`). Do not treat that return byte as proof that a texture was accepted.

**Conformance expectations (static).** Use an 8×6 `cGZBuffer` source with distinguishable texels, normal x87
24-bit nearest-even mode, a destination large enough for the listed coordinates, and tiling cleared. Prefill
`outRect` with four unrelated sentinel dwords; none may influence these success results. Values in fixed-point
columns are hexadecimal dwords; rectangle/sample coordinates are decimal.

| x,y | sx,sy | stored forward scales | du,dv | ox,oy | outRect | Sample at first destination pixel |
|---|---|---|---|---|---|---|
| 3,5 | 1,1 | `00010000,00010000` | `00010000,00010000` | `00000003,00000005` | (3,5,11,11) | dest (3,5) → source (0,0) |
| 3,5 | 2,0.5 | `00020000,00008000` | `00008000,00020000` | `00018000,000A0000` | (3,5,19,8) | dest (3,5) → source (0,1) |
| 3,5 | 0.5,2 | `00008000,00020000` | `00020000,00008000` | `00060000,00028000` | (3,5,7,17) | dest (3,5) → source (1,0) |
| 3,5 | 0.75,0.75 | `0000C000,0000C000` | `00015555,00015555` | `0003FFFF,0006AAA9` | (3,5,9,9) | dest (3,5) → source (0,0) |
| 3.5,5.5 | 1,1 | `00010000,00010000` | `00010000,00010000` | `00000004,00000006` | (4,6,12,12) | dest (4,6) → source (0,0) |
| 3.5,5.5 | 2,2 | `00020000,00020000` | `00008000,00008000` | `0001C000,0002C000` | (3,5,19,17) | dest (3,5) → source (0,0) |

For tiling, first use the first source-setup row, then SetBltTiling with `(2,1,6,4)`. Expect tile width 4, height 3,
source pointer biased by `2*2 + 1*sourceStride` for a 16-bit source, and offsets `(1,1)`. Destination
`(3,5),(4,5),(7,5)` must sample original source texels `(2,1),(3,1),(2,1)`. The input tile rect is unchanged.
ClearBltTiling restores the first row's plain mapping. Also check that null source returns 0 and leaves the
sentinel output unchanged, and a source refusing `0x20732180` returns 1 with no output write.

### 1.3.2 CALL sites and exact return addresses (SP6b) [exact, byte-backed]

Every address listed as a call site in [span-render-565.md §1.1](span-render-565.md#11-native-call-boundary-re-span565-bridge4-exact)
is the **first byte of the CALL instruction**, not a return address. All seven are three-byte indirect calls
through the renderer vtable; the instruction immediately after each is the exact return address below. The
slot offsets in those bytes are `+0x14,+0x14,+0x14,+0x10,+0x08,+0x0C,+0x04`, respectively. The target for 4444
comes from its table `0x00AC185C` (section 1.1), not from a presumed 565 target address.

| Path | CALL site | Exact return address | Slot | 4444 target |
|---|---|---|---|---|
| tiled texture | `0x008304D1` | `0x008304D4` | 5 | `0x008480C0` |
| stretched texture | `0x0083053B` | `0x0083053E` | 5 | `0x008480C0` |
| plain texture | `0x00830591` | `0x00830594` | 5 | `0x008480C0` |
| alpha gradient | `0x008305E7` | `0x008305EA` | 4 | `0x00847F10` |
| opaque gradient | `0x008305FC` | `0x008305FF` | 2 | `0x00847D30` |
| alpha colour | `0x00830627` | `0x0083062A` | 3 | `0x00847E40` |
| solid colour | `0x00830647` | `0x0083064A` | 1 | `0x00847CE0` |

Classify a renderer-entry return address by equality with this table; accepting every address 1..7 bytes after
a site is unnecessarily broad. As a classifier conformance check, each exact return must select its row;
the CALL site itself and site+1,+2,+4,+5,+6,+7 must not. Context identity/active-fixture scope is still required
to separate constructed calls from shipped calls (the preceding counting rule). Static call-site proof does
not establish new shipped-hit counts or reclassify the earlier 378 constructed cases as shipped evidence.

## 2. Shared conventions

**Pixel.** 16 bits, A in bits 12-15, R 8-11, G 4-7, B 0-3 (masks `0xF000/0x0F00/0x00F0/0x000F`).

**Span record** (8 bytes). Two dwords `start`, `end`, each packed as `(y << 16) | x` with unsigned 16-bit x and y.
Width is `end - start` computed on the whole dwords, so a span is one row: `end` is `start` plus the pixel count
(exclusive end). A span with width 0 writes nothing.

**Target descriptor** (argument `buffer`): dword `+0` = base address of pixel (0,0), dword `+4` = row stride in
bytes. Pixel (x, y) is at `base + y*stride + 2*x`. No clipping is done in this class: spans are assumed
pre-clipped by the caller (`DrawPreclippedSpanList`, view3d.md 7.1).

**Span list loop.** Slots 1-4 take `(spans, count, buffer, ...)` and process `count` spans in order with a
decrement-then-test loop: `count` must be at least 1 (a count of 0 is not guarded; see U1).

## 3. RenderSolid (`0x00847CE0`)

Arguments `(spans, count, buffer, colour)`; `colour` is already a 4444 pixel (low 16 bits used). For each span,
fills `width` pixels from (x, y) with `colour` (helper `0x0090FFC8`, a 16-bit fill: one leading word if the
address is not 4-byte aligned, then dword pairs, then one trailing word). The pixel is stored verbatim, alpha
nibble included.

## 4. RenderAlpha (`0x00847E40`) - constant colour over the destination

Arguments `(spans, count, buffer, colour, alpha)`.
- Weight: `a = ((alpha >> 27) + 1) >> 1` on the unsigned 32-bit `alpha` (only its top 5 bits matter), so
  `a` is 0..16. With the alpha byte in bits 24-31: `a = ((A >> 3) + 1) >> 1`; A = 0x00..0x07 gives 0,
  0xF8..0xFF gives 16.
- Per pixel, for each of R, G, B (4-bit `s` from `colour`, 4-bit `d` from the destination):
  `out = floor((16*d + a*(s - d) + 8) / 16)`. (R and B are computed together in one register as
  `((sRB - dRB)*a + 16*dRB + 0x808) >> 4`, G alone as `((sG - dG)*a + 16*dG + 128) >> 4` on the in-place
  fields; the low field never borrows into the high one because `16*d + a*(s-d) + 8` stays in 8..248.)
- The output alpha nibble is always `0xF`, whatever the destination or `colour` alpha was.
- Worked example: d = `0x8000` (R=G=B=0), colour `0x0F84`, alpha `0x80000000` (a = 8):
  R = floor((0 + 8*15 + 8)/16) = 8, G = floor((64+8)/16) = 4, B = floor((32+8)/16) = 2, result `0xF842`.

## 5. RenderGradient (`0x00847D30`) - linear colour ramp, opaque

Arguments `(spans, count, buffer, grad)`. `grad` is 12 signed dwords: `+0x00..+0x0C` start values for channels
0-3, `+0x10..+0x1C` x steps, `+0x20..+0x2C` y steps. Channels 0, 1, 2 are B, G, R in 8.16 fixed point (8-bit
value in bits 16-23); channel 3 is alpha and is ignored here.
- At each span's (x, y) the start value is `c = start + dx*x + dy*y` (32-bit wrapping integer maths).
- B: the span start is then shifted right logically by 4 and the per-pixel step is `dx_B` shifted right
  arithmetically by 4, so B accumulates with the low 4 bits of its step dropped (B drifts versus the exact ramp).
- G: start and step unchanged.
- R: start shifted left 4, then OR `0xF0000000`; step `dx_R << 4`.
- Each pixel: the 16-bit pixel is bits 16-31 of `(Racc & 0xFF000000) + (Gacc & 0x00F00000) + (Bacc & 0x000F0000)`,
  i.e. each channel is the top 4 bits of its 8-bit integer part (truncation, no rounding, no dither), and the
  alpha nibble is the top nibble of the R accumulator, which is `0xF` while R stays within 0..255.99 along the
  span. If R leaves that range mid-span the carry/borrow lands in the alpha nibble (U2).
- Then each accumulator adds its step and the next pixel (x+1) is written.

## 6. RenderAlphaGradient (`0x00847F10`) - linear colour and alpha ramp over the destination

Arguments `(spans, count, buffer, grad)`, same 12-dword layout as section 5, all four channels used.
- B, G, R accumulators are set up exactly as in section 5 (including the B >> 4 drift; R uses `<< 4` but no OR).
- Alpha accumulator: `(startA + dxA*x + dyA*y) * 0x88`, step `dxA * 0x88` (32-bit wrapping). Per pixel
  `a = acc >> 27` (unsigned). If `W` is the raw 8.16 dword at that pixel, the exact map is
  `a = U32(136*W) >> 27`, with `U32` meaning the low 32 bits. For the represented real value
  `A = W/65536` in 0..255, this simplifies to `floor(17*A/256)`, giving 0..16. It is a different map from
  RenderAlpha (section 4); see section 8. Outside that range the wrapped map, not the simplified floor, applies.
- Source colour per pixel: R = bits 24-27 of the R accumulator, G = bits 20-23 of G, B = bits 16-19 of the
  shifted B accumulator (top nibbles of the 8-bit integer parts); then the packed blend below with this `a`.
  Output alpha nibble `0xF`. The R accumulator's top nibble is masked off, so out-of-range R does not reach
  alpha here.

### 6.1 Alpha domain and packed arithmetic (SP1) [exact, instruction bytes]

The normal colour domain is 0..255 per channel (0..1 before the native gradient setter's conversion), but it is
**not a checked precondition** of this renderer or its setter. For an otherwise valid span/destination/gradient,
every 32-bit alpha start and step has the wrapping behaviour here. Do not reject or saturate a gradient because
its alpha crosses 0 or 255. `SetGradientTransform` uses the same four-channel conversion for alpha as for RGB,
with no range comparison (`0x0082FDD0`–`0x0082FE43`; shared conversion in
[span-render-1555.md §4.1](span-render-1555.md#41-gradient-record-0x30-bytes-shared-with-5)). This proves the
setter's applicability to this context; it does not prove that shipped UI paths request such gradients.

The renderer first forms `W = U32(startA + dxA*x + dyA*y)` and `acc = U32(136*W)`, then adds the wrapped
`136*dxA` after each pixel (`0x00847FAF`–`0x00847FCA`, `0x0084807C`–`0x0084808D`). Thus `a` is always **0..31**;
17..31 are reachable. RenderAlpha cannot reach them: it reduces the top five bits a second time to 0..16
(`0x00847E50`–`0x00847E56`).

Let `S` be the source RGB nibbles packed as `0x0RGB` and `D` the destination word. Define
`sRB = S & 0x0F0F`, `dRB = D & 0x0F0F`, `sG = S & 0x00F0`, `dG = D & 0x00F0`.
All subtraction, multiplication and addition below retain only their low **32** bits; all right shifts are
**logical**. The native order is:

1. Subtract `dRB` from `sRB`, multiply by `a`, then add `16*dRB` and `0x808`, obtaining `rb`.
2. Subtract `dG` from `sG`, multiply by `a`, then add `16*(dG+8)`, obtaining `g`.
3. Shift `rb` and `g` right by 4. OR the first with `0xF0F0`, OR the second with `0xFF0F`, AND those results,
   and store only the low 16 bits. Equivalently the word is
   `0xF000 | ((rb >> 4) & 0x0F0F) | ((g >> 4) & 0x00F0)`.

Anchors: source/destination field extraction `0x00847FF0`–`0x0084801B`; unsigned weight extraction
`0x00848022`; 32-bit subtract/multiply/add `0x0084801E`–`0x0084804E`; shifts, masks and word store
`0x00848054`–`0x0084806C`. There is **no mask between the packed subtraction and the shift**. For a <= 16,
this agrees with the separate-channel rule in section 4. For a > 16 the blue numerator can carry or borrow
into the red calculation; keeping each channel's low nibble separately is not equivalent. Green remains
separate; alpha is always F even when a packed intermediate is negative or overflows.

**Discriminating conformance vectors.** Use one span at (0,0) of width 1, count 1, with all gradient steps zero.
Set raw RGB starts to `(B,G,R) = (0, 0x00700000, 0)`, i.e. source `0x0070`. For each row the raw alpha start is
`A*65536` (A below is an integer in channel units, not a normalized float). The expected word is derived from
the instruction-width rule above. The last column shows the distinguishable result of rounding each channel
separately and retaining its low nibble; it is not an accepted alternative.

| A | a | Destination | Expected word | Separate channels |
|---|---|---|---|---|
| 256 | 17 | `F859` | `FF7F` | `F07F` |
| 272 | 18 | `F455` | `FF7F` | `F07F` |
| 287 | 19 | `F853` | `FE7F` | `FF7F` |
| 302 | 20 | `F253` | `FF8F` | `F08F` |
| 317 | 21 | `F852` | `FD8F` | `FE8F` |
| 332 | 22 | `F452` | `FE8F` | `FF8F` |
| 347 | 23 | `F852` | `FC8F` | `FD8F` |
| 362 | 24 | `F152` | `FF8F` | `F08F` |
| 377 | 25 | `F851` | `FB8F` | `FC8F` |
| 392 | 26 | `F451` | `FD8F` | `FE8F` |
| 407 | 27 | `F851` | `FA8F` | `FB8F` |
| 422 | 28 | `F251` | `FE9F` | `FF9F` |
| 437 | 29 | `F851` | `F99F` | `FA9F` |
| 452 | 30 | `F451` | `FC9F` | `FD9F` |
| 467 | 31 | `F851` | `F89F` | `F99F` |

Additional boundary/carry vectors, again with zero steps. Set each raw RGB start to its source nibble times
`0x00100000`:

| Source S | Destination | Raw alpha W | acc | a | Expected word |
|---|---|---|---|---|---|
| `0070` | `8859` | `00000000` | `00000000` | 0 | `F859` |
| `0070` | `8859` | `00FF0000` | `87780000` | 16 | `F070` |
| `0000` | `880F` | `01000000` | `88000000` | 17 | `FF0F` |
| `070F` | `8000` | `01000000` | `88000000` | 17 | `F800` |
| `0070` | `F851` | `FFFF0000` (-1.0) | `FF780000` | 31 | `F89F` |
| `0070` | `8859` | `01E20000` (482.0) | `00100000` | 0 | `F859` |
| `0070` | `8859` | `80000000` | `00000000` | 0 | `F859` |

In the `0000`/`880F` borrow case, `rb = FFFFFFF9`, logically shifted to `0FFFFFFF`; the mask/store yields
`FF0F` rather than the separate-channel `F00F`. In the `070F`/`8000` carry case, `rb = 00008007`, yielding
`F800` rather than `F700`. These are static conformance expectations, not newly recorded native game results.

Two-pixel step/order checks use the same constant source `0070` and equal initial destinations at both pixels.
With destinations `F859`, raw alpha start `00FF0000` and x step `00010000`, expect `F070,FF7F` (weights 16,17).
With destinations `F851`, raw alpha start `01E10000` and the same step, expect `F89F,F851` (weights 31,0, because
the accumulator wraps). Apply the step **after** writing the current pixel; neither boundary clamps.

## 7. RenderBlt (`0x008480C0`) - kernel dispatch

Arguments `(p1, p2, rectA, rectB, src, mode)` (6 dwords, thiscall, `RET 0x18`; section 1.1): `rectA`, `rectB` point to 24-byte records, `src` to a 32-byte
source descriptor, `mode` an int. It copies the three records by value and calls
`kernel(p1, p2, *rectA, *rectB, *src)` (cdecl, 88 bytes of arguments), where
`kernel = table[src.format * 2 + src.flag][mode]`: `src.format` is the dword at `src+0x10` (0..6),
`src.flag` the byte at `src+0x1C` (0/1). The row-pointer array is at `0x00AC2100` (14 entries); row `r` is the
12-entry array at `0x00AC20D0 - 0x30*r`, so the 168 entries occupy `0x00AC1E60`-`0x00AC20FF`. Neither index is
range-checked (U3).

Kernel arguments as the kernels read them (`p1`, `p2` and the three records):
- `p1` = span list, `p2` = span count (same span loop as section 2).
- `rectA` (24 bytes) = destination: `+0` base, `+4` stride (bytes). `+8..+0x14` not read by any examined kernel.
- `rectB` (24 bytes) = source image: `+0` base, `+4` stride, `+8` width, `+0x0C` height (the last two read only
  by tiled kernels). `+0x10`, `+0x14` not read by any examined kernel.
- `src` (32 bytes): `+0x00` u0, `+0x04` v0, `+0x08` du/dx, `+0x0C` dv/dy, `+0x10` source format (row), `+0x14`
  constant colour/alpha (ARGB 8888), `+0x18` colour key, `+0x1C` keyed flag (byte), other bytes unread.

## 8. Kernel table: rows and columns

**Rows** (`format*2 + keyed`). The source format is identified from each row's plain `Blt` kernel (how it loads
and splits the source pixel):

| format | source pixel | read as | Mac name (RGBFormat masks) |
|---|---|---|---|
| 0 | A4R4G4B4, 16-bit | copied verbatim | `61440,3840,240,15,2,4` |
| 1 | A1R5G5B5, 16-bit | A = bit 15 | `32768,31744,992,31,2,5` |
| 2 | X1R5G5B5, 16-bit | A forced opaque | `0,31744,992,31,2,5` |
| 3 | R5G6B5, 16-bit | A forced opaque | `0,63488,2016,31,2,5` |
| 4 | A8R8G8B8, 32-bit | A = bits 24-31 | `4278190080,16711680,65280,255,4,8` |
| 5 | X8R8G8B8, 32-bit | A forced opaque | `0,16711680,65280,255,4,8` |
| 6 | A8, 8-bit (alpha only) | R = G = B = max | `255,0,0,0,1,0` |

**Columns** (`mode`): geometry x operation, `mode = 4*geometry + op`.
- geometry 0 = direct (source pixel at the same (x, y) as the destination pixel), 1 = stretch, 2 = tiled.
- op 0 = copy, 1 = constant alpha, 2 = modulate, 3 = modulate + alpha.
- Evidence: every kernel in columns 4-7 carries the extra 16.16 shifts of the stretch walk and every kernel in
  columns 8-11 a `DIV` (tile wrap); columns 1/5/9 take the alpha nibble of `src+0x14`, columns 2/3/6/7/10/11
  use the MMX multiply, columns 3/7/11 additionally blend with the destination. This matches, one for one, the
  12 Mac method names `Blt, BltConstantAlpha, BltModulate, BltModulateAlpha` and their `Stretch`/`Tiled` forms
  (Mac `cBlitter<Dst, Src, keyed>`); the Mac `BltRect` has no Windows column.
- Two table entries are outside this owner range because identical code was folded: row 0 column 4
  (`0x00833730`, in the 1555 range) and row 0 column 8 (`0x00842C50`, in the 565 range); both are a raw 16-bit
  copy that does not depend on the pixel format. So this range holds 166 kernels + 5 methods + the helper
  `0x0084BD40` = 172 functions.

## 9. Kernel semantics

### 9.1 Geometry (source texel for destination (x, y))
- Direct: source pixel `(x, y)` of `rectB` (the caller pre-offsets `rectB.base`). Verified `0x00848160`.
- Stretch: per span, `v = (dvdy*y + v0) >> 16` (unsigned), source row `base + v*stride`; `u = dudx*x + u0`, each
  pixel reads texel `u >> 16` (unsigned) then `u += dudx`. x and y are the absolute destination coordinates; no
  clamping. Verified `0x00833730`.
- Tiled: per span, `tx = (x + u0) mod width`, `ty = (y + v0) mod height` (unsigned 32-bit division; here u0, v0
  are integer pixel offsets, not 16.16); each pixel reads `(tx, ty)`, then `tx += 1` and wraps to 0 on reaching
  `width`. Verified `0x00842C50`.

### 9.2 Conversion to 4444 (copy and constant-alpha ops)
Each channel keeps the top 4 bits of the source channel (truncation, no rounding, no dither):
A1R5G5B5: A = 0xF if bit 15 else 0, R = bits 11-14, G = bits 6-9, B = bits 1-4. X1R5G5B5: same with A = 0xF.
R5G6B5: R = bits 12-15, G = bits 7-10, B = bits 1-4, A = 0xF. A8R8G8B8: A = bits 28-31, R 20-23, G 12-15,
B 4-7. X8R8G8B8: same with A = 0xF. A8: A = bits 4-7 of the byte, RGB = `0xFFF`. 4444: unchanged.
Verified at the plain `Blt` kernels `0x00849C50`, `0x0084BBD0`, `0x0084DAB0`, `0x0084F9E0`, `0x00851700`,
`0x008533A0`.

### 9.3 Conversion to 8888 (modulate ops)
Each channel is widened by bit replication: 4-bit `n` -> `n*17`; 5-bit `v` -> `(v << 3) | (v >> 2)`; 6-bit
`g` -> `(g << 2) | (g >> 4)`; 1-bit alpha -> 0xFF or 0; formats without alpha -> A = 0xFF; A8 -> A = byte,
R = G = B = 0xFF; 8888 unchanged. Verified at column 2 of rows 0, 2, 4, 6, 8, 10, 12
(`0x00848260`, `0x00849DE0`, `0x0084BDB0`, `0x0084DC20`, `0x0084FB60`, `0x00851860`, `0x008534C0`).

### 9.4 Operations
- **Copy** (op 0): write the 9.2 conversion. The destination is not read. Alpha comes from the source.
- **Constant alpha** (op 1): `a = src.colour >> 28` (top nibble of the alpha byte of `src+0x14`, 0..15). The
  source is converted per 9.2, its alpha ignored, then the helper `0x0084BD40` blends each of R, G, B:
  `out = floor((16*d + a*(s - d) + 8) / 16)`, output alpha `0xF`. Because `a` tops out at 15, a fully opaque
  constant alpha does not reproduce the source: s = 15 over d = 0 gives floor(233/16) = 14. (Contrast
  RenderAlpha, whose weight reaches 16.)
- **Modulate** (op 2): widen per 9.3 to `C`, then per byte channel (A, R, G, B)
  `M = (C * K + 128) >> 8` with `K` the matching byte of `src.colour` (MMX word multiply, rounding constant
  `0x0080` from `0x00AC1B80`); write the top nibble of each M byte (A from M too). The destination is not read.
- **Modulate + alpha** (op 3): M as above, then blend R, G, B (top nibbles of M) over the destination with
  `a = M >> 28` by the op-1 formula; output alpha `0xF`. Verified `0x00848380`.
- **Keyed rows** (odd rows): before anything else each source texel is compared with `src+0x18`; if
  `(texel & rgbmask) == key` the destination pixel is left untouched. `rgbmask` is the format's colour bits:
  4444 `0x0FFF`, 1555 and X555 `0x7FFF`, 565 `0xFFFF`, 8888 `0xFFFFFF` (`0x00848DB0`, `0x0084AB70`,
  `0x0084CAD0`, `0x0084E990`, `0x008507C0`). For A8 the mask is empty, so the test degenerates to "key == 0":
  with a zero key **nothing** is drawn, with any other key every texel is drawn (`0x00854070`, `0x00854190`).
  The key compare is present in every column of the keyed rows sampled (rows 1, 3, 13; columns 0-3, 5, 9).

### 9.5 Verification level
Fully read (round 1): the five methods, the helper, row 0 columns 0-5 and 8, the Blt kernel of every format, the
key test of column 0 in all seven keyed rows (mask per format; X8R8G8B8 `0x008524C0` masks `0xFFFFFF`), the 8888
widening of every unkeyed row.

All 168 table entries (round 2) were then checked by **executing the image bytes** against the model of 9.1-9.4,
which closes the old feature-sweep gap (U4). Method and criteria, so a successor can repeat it:
1. Byte identity. Each kernel's bytes were disassembled independently of Ghidra (Capstone, from
   `bin/SimCity4.exe`) and compared with the export's instruction list: 168 functions, 0 mismatches in
   bytes or mnemonic (Ghidra `JZ/JNZ/JC` = `je/jne/jb`); the only bytes the export omits are 91 alignment
   fillers inside loops (`lea r,[r]`, `nop`, `mov edi,edi`), which have no effect.
2. Text normalization is not enough. With in-function branch targets renamed to ordinal labels and all
   immediates, displacements and widths kept, no two kernels are textually identical, and kernels of one column
   differ from row 0 by 5-91 lines (difflib, longer side of each hunk): register allocation, instruction scheduling and stack offsets (one more or one
   fewer saved register) differ between rows, so a textual "same template, different conversion" proof fails.
   The constant inventory (AND/CMP/TEST immediates) does follow the row: 4444 rows `0xF..0xF000`, 1555/X555 add
   `0x1F 0x3E0 0x7C00` and the replication mask `0x70707`, 565 `0x7E0 0xF800` and `0x70007`, keyed rows add
   their key mask (`0xFFF`, `0x7FFF`, `0xFFFFFF`; the 565 key uses all 16 bits and A8 has no mask, so neither adds
   a constant), and only the 1555 rows hold `0x1`/`0xFF000000` (the alpha bit).
3. Execution. A small x86-32 interpreter covering exactly the 35 mnemonics the kernels use (integer ALU with
   ZF/CF, `DIV`, `IMUL`, the MMX `MOVD/MOVQ/PUNPCKLBW/PXOR/PMULLW/PADDW/PSRLW/PACKUSWB/EMMS`) runs each kernel
   with the RenderBlt argument block (cdecl, 88 bytes) and random contents: 1-4 spans of 1-25 pixels, random
   destination pixels, random texels (10 % forced to all-zero or all-one), constant colour random or one of
   `0xFFFFFFFF, 0, 0x80808080, 0xF0FFFFFF`, keys equal to a masked texel 80 % of the time (A8: key 0 half the
   time), random entry registers, random values in every field the spec calls unread (U6). Images 5x3, 37x11
   and 64x16 with row padding; stretch with u0 < 2.0, du/dv < 1.0 texel; tiled with offsets 0..999 and random
   du/dv (which tiled kernels must ignore). The result is compared pixel by pixel with 9.1-9.4, including the
   untouched pixel either side of each span.
   Result: 150 trials per kernel, **168/168 kernels, 747,927 pixels, 0 mismatches**; every kernel returns with
   ESP restored and EBX/ESI/EDI/EBP preserved. Exhaustive texel sweep (all 65,536 16-bit texels, all 256 A8
   bytes) for columns 0-3 of rows 0, 2, 4, 6, 12 and column 0 of rows 1, 3, 5, 7, 13: 25 kernels, 1,312,000 pixels, 0 mismatches (the 9.2/9.3
   conversions and the per-format key masks hold for every texel value).
4. Sensitivity. The same harness with one rule of the model changed fails at once: blend rounding `+7`
   instead of `+8` (78 kernels fail at 4 trials), constant-alpha weight `a+1` (39), modulate without `+128` (71),
   4444 key mask `0xFFFF` (all 12 keyed 4444 kernels at 40 trials), tiled start without `u0` (52).

So every rule in 9.1-9.4 is verified on all 168 kernels for the input ranges above. What execution does not
cover is listed in U5 and U7.

## 10. Unknowns

- U1. Count 0 in slots 1-4: the loop decrements before testing, so 0 would run 2^32 iterations; whether any
  caller passes 0 is not checked.
- U2. RenderGradient with R outside 0..255 along a span corrupts the alpha nibble (section 5); whether callers
  ever produce such ramps is not checked.
- U3. RenderBlt does not validate `src.format`, `src.flag` or `mode`.
- U4. (Closed in round 2.) The 144 kernels that were only feature-swept are now verified by execution against
  9.1-9.4 (section 9.5); none differs from the model.
- U5. Whether the stretch walk ever lets `u >> 16` or `v` run past the source (no clamp in the kernel) depends
  on the caller's u0/du; not checked here.
- U6. Fields `rectA+8..+0x14`, `rectB+0x10..+0x14` and the bytes of `src` beyond those listed are unread by the
  examined kernels; their meaning belongs to the caller (`cGZDrawContextRenderToBuffer`). Round 2: randomizing
  them (and u0/v0/du/dv for direct kernels, du/dv for tiled kernels, `src+0x10` and bytes `src+0x1D..0x1F`)
  never changed any kernel's output.
- U7. Not covered by the execution check: spans wider than 25 pixels or beyond x, y = 65,535; stretch steps of
  1.0 texel or more and u/v accumulators that wrap 2^32; tiled `x + u0` wrapping 2^32; zero width or height in
  tiled kernels (the `DIV` would fault); negative strides. The model's formulas are expected to hold there, but
  only by the reading of 9.1.

## 11. Coverage proposal

- specified: `nGZDrawSoftRender::cSpanRender4444` (whole owner range, 172 functions, 51,809 bytes): 5 methods +
  helper + ~24 kernels read in full; all 168 table entries (166 in range + 2 folded) verified by execution
  against 9.1-9.4 (section 9.5); remaining limits U1-U3, U5-U7.
- ABI: the class is internal to `cGZDrawContextRenderToBuffer`. Proposed `docs/abi/vtables.tsv` rows for
  `0x00AC185C` (section 1.1 table) are in lane re-spanbridge4444-4's `build-runs/abi-rows-re-spanbridge4444-4.tsv`;
  not reconciled into docs/abi.
