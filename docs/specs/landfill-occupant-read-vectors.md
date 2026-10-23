# Landfill occupant reader and position vectors (LG32/LG33)

Companion to [landfill §6.4 and §7.3](landfill.md) and [foundations §10.3](foundations.md).
These are independently authored literal inputs and **proposed** expectations derived from the original's
instructions. No vector was executed against the game or an implementation in this analysis lane. “Checked”
below means instruction-checked static evidence or an explicitly identified prior measurement, not native
acceptance of the proposed matrix. No native output is an input to these vectors.

## 1. Literal input, state and call notation

`B` is exactly these 57 bytes, in increasing offset order (hex):

| Offset | Bytes | Typed meaning |
|---|---|---|
| 0 | `03 00` | landfill version 3 |
| 2 | `04 00` | base version 4 |
| 4 | `02 01` | removal word `0x0102` |
| 6 | `00 F4` | highlight 0, flags `0xF4` |
| 8 | `2B BD 11 6A` | type `0x6A11BD2B` |
| 12 | `FF 00 00 00` | manager rect |
| 16 | `02 00` | exemplar version 2 |
| 18 | `02 00` | plain version 2 |
| 20 | `00 00 00 00` | count 0 |
| 24 | `00 00 00 00 00 00 00 00 00 00 00 00` | null parent group/type/instance |
| 36 | `10 00 F1 02` | tile `0x02F10010` |
| 40 | `F9 FF FF FF` | orientation -7 |
| 44 | `00 00 C4 C1` | x = -24.5 |
| 48 | `00 00 44 41` | y = 12.25 |
| 52 | `00 00 20 42` | z = 40 |
| 56 | `00` | empty false |

Every row below starts from a separate object in state `S`: removal `0x7788`, highlight 3, flags `0x04`,
type `0x6A11BD2B`, rect `11 22 33 44`; tile `0x02F10000`, orientation 9, position `(-1,-2,-3)`,
empty true, bounds `(-9,-2,-11)` … `(7,14,5)`; one own Uint32 property `0x01020304 = 0x11223344`,
null parent. This is a proposed precondition, not a claim that every field can be set through the published
native interfaces. The original LG33 fixture instead had an empty own holder and null parent.

`B[a:b]` means the literal slice including a, excluding b. “Replace at n” changes only the stated bytes.
`U16@n`, `U32@n`, `I32@n`, `F32@n`, `U8@n` denote stream calls and their starting byte offsets.
Each annotation specifies one primitive call, not a bulk read. `G` denotes `GetError()` with no byte effect.
The stream starts good and uses the atomic, sticky segment-stream contract of foundations §2.1–§2.2.
Consumed counts are relative to the start of `B`, not relative to the nested holder. Numerical bound
expectations assume round-to-nearest; this lane has not measured the game's FPU environment.

Full call order `C`:

1. Landfill: `U16@0`; base: `U16@2,U16@4,U8@6,U8@7,U32@8,U8@12,U8@13,U8@14,U8@15`.
2. Exemplar: `U16@16`; plain: `U16@18,U32@20`, clear own entries, `G` (plain return).
3. Exemplar: `U32@24,U32@28,U32@32`, process null key, `G` (exemplar return).
4. Base: `G` (ignores exemplar return). Landfill:
   `U32@36,I32@40,F32@44,F32@48,F32@52,U8@56`, derive empty, bounds, `G` (landfill return).

The reader bodies do not test the primitive results. They test own versions, plain return at exemplar,
base return at landfill, and the error checks above. No extra `G` occurs on a version-rejection exit.

## 2. Version and complete-input vectors

All inputs in this table are 57 bytes. All have final stream error 0. `H` means base header effects:
removal `0x0102`, status byte 2 (public highlight 0), flags `0xF4`, type `0x6A11BD2B`, rect `FF 00 00 00`.
`T` means tile `0x02F10010`, orientation -7, position `(-24.5,12.25,40)`, empty false,
bounds `(-32.5,12.25,32)` … `(-16.5,28.25,48)`.

| ID / literal change to B | Expected nested calls / return | Top AL / consumed | Expected retained or changed state |
|---|---|---|---|
| V2: none | `C`; plain true, exemplar true, base true | true / 57 | H + T; own entries cleared, parent null |
| V0: offset 16 → `00 00` | Exemplar false at cursor 18; no plain read or key; base `G` → true; tail starts 18 | true / 39 | H; own entry and parent retained; tail state M18 below |
| V1: offset 16 → `01 00` | Same as V0 | true / 39 | Same as V0 |
| V3: offset 16 → `03 00` | Same as V0 | true / 39 | Same as V0 |
| P1: offset 18 → `01 00` | Exemplar accepts 2, plain false at 20; exemplar tests false and returns false without key or `G`; base `G` → true; tail starts 20 | true / 41 | H; own entry and parent retained; tail state M20 below |
| P3: offset 18 → `03 00` | Same as P1 | true / 41 | Same as P1 |
| O2: offset 0 → `02 00` | `U16@0` only; no base/holder/tail/`G` | false / 2 | S unchanged |
| B3: offset 2 → `03 00` | `U16@0,U16@2`; base false, landfill tests it; no holder/tail/`G` | false / 4 | S unchanged |

Misaligned tails are byte re-interpretation, not recovery:

| State | Actual tail calls | Stored values and bounds |
|---|---|---|
| M18 | `U32@18,I32@22,F32@26,F32@30,F32@34,U8@38,G` | tile 2; orientation 0; position f32 words `00000000,00000000,00100000`; byte `F1` → empty false; bounds `(-8,0,-8)` … `(8,16,8)` |
| M20 | `U32@20,I32@24,F32@28,F32@32,F32@36,U8@40,G` | tile 0; orientation 0; position f32 words `00000000,00000000,02F10010`; byte `F9` → empty false; bounds `(-8,0,-8)` … `(8,16,8)` |

Hex words in the last table are raw binary32 encodings, not integer-to-float conversions. Adding/subtracting
8 rounds the very small positive z values to ±8 in these cases. No lookup of the apparent tile ID is made
by this tail read. Bytes 39–56 (M18) or 41–56 (M20) remain unread. An owner that calls another Read on the
same unbounded stream starts there; this body never seeks to 57.

## 3. Primitive errors and truncation

These vectors use the literal input `B[0:n]`, not a writer's output. “Attempt suffix” means the calls still
occur after the first failure, but sticky errors consume no more bytes and preserve their destinations.
The cursor stops at the first primitive that cannot fit; the trailing 1–3 bytes of a short multi-byte field
also remain unread. Results involving uninitialised stack storage are deliberately partial expectations.

| ID / input | First failure, attempted continuation | Top result / consumed / error | State guarantee |
|---|---|---|---|
| E-before / n=15 | Fourth rect byte `U8@15` fails. Base still invokes exemplar `U16@15`; its failed version destination contains reused stack data, not a supplied version. If it happens to compare equal to 2, plain is entered with failed reads too. | false **if Read returns** / 15 / 1 | H through type; rect `FF 00 00 44`; no landfill tail or bounds update. Holder outcome / finite call count not determined by input bytes. |
| E-count / n=20 | Plain version accepted; `U32@20` count fails, then own entries are cleared. The count temporary is not initialised, so the entry loop is not bounded by this input. | false **if Read returns** / 20 / 1 | H; own clearing occurs; parent retained because exemplar tests the eventual plain false. No landfill tail. No portable allocation/termination expectation. |
| E-key0 / n=24 | Plain true at 24 after clearing; `U32@24` fails; calls at same cursor for type/instance fail. Zero-initialised key is processed, then exemplar `G` → false, base `G` → false. | false / 24 / 1 | H, own cleared, parent null; tile/orientation/position/empty/bounds all retained from S. A previously nonnull parent would be cleared here too. |
| E-key1 / n=28 | Group zero read; type at 28 fails; instance attempted at 28 fails; same key processing and two error checks as E-key0. | false / 28 / 1 | Same state guarantee as E-key0. |
| E-tile / n=36 | Base true after complete holder. Tile at 36 fails; orientation, x,y,z,empty calls all attempted at 36. Derive empty, update bounds, landfill `G` → false. | false / 36 / 1 | H, own cleared, parent null; tile/orientation/position retained S; bounds recomputed from S position; empty not specified. |
| E-y / n=50 | Tile/orientation/x succeed. `F32@48` cannot fit (2 bytes left), z and empty attempted at 48 also fail. Derive empty, bounds, `G`. | false / 48 / 1 | H; own cleared, parent null; tile and orientation from T, position `(-24.5,-2,-3)`; bounds `(-32.5,-2,-11)` … `(-16.5,14,5)`; empty not specified. |
| E-last / n=56 | All through position succeed; `U8@56` fails; derive empty, bounds, `G`. | false / 56 / 1 | H + T except empty not specified; own cleared, parent null; new bounds still computed. |
| E-exact / n=57 | No failure; `C`. | true / 57 / 0 | H + T, own cleared, parent null. |

Before a successfully read outer/base/holder version, truncation is not equivalent to supplying a bad version:
the destination may hold old stack bits. In particular `B[0:0]`, `B[0:1]`, `B[0:3]`, `B[0:17]` and
`B[0:19]` do not justify a deterministic nested-call count or retained-holder oracle. They do set the sticky
error on the first attempted unavailable primitive; its failed destination is unchanged. No implementation
is required to reproduce arbitrary stack contents. For a sticky error already set before entry, the first
outer version read is likewise a no-op with an unspecified version comparison; do not claim immediate
version rejection from bytes that were never read.

Failed highlight and empty reads use reused stack byte storage (base `0x005E8D93`–`0x005E8DA7`,
landfill `0x004775B6`–`0x004775C8`), not their old object values. This is why E-last does not assert
empty retained or false. For a nonzero successfully read group followed by a short key, zero initialisation
does not suppress resource lookup: processing still uses that nonzero partial key before the error check.
The resource-manager outcome requires a separate controlled resource fixture; it is not part of this matrix.

## 4. Typed position and return observations

Proposed position checks use the occupant slot 9 as a void setter with a pointer to three f32 words. Use
slot 8's output vector and slot 10's two output vectors to compare effects; do not score a typed bool
constructed from residual EAX/AL as acceptance. Every vector uses a separate valid object.

| ID | Input vector bytes (x / y / z) | Expected typed outputs | Static residual register observation |
|---|---|---|---|
| S0 | `00 00 B1 42 / 00 00 80 C0 / 00 80 F0 42` | position `(88.5,-4,120.25)`; bounds `(80.5,-4,112.25)` … `(96.5,12,128.25)` | EAX `42F08000`, AL `00`; no Boolean return contract |
| S1 | `00 00 B1 42 / 00 00 80 C0 / 01 80 F0 42` | exact input words retained; x/y bounds as S0; z bounds f32 words `42E08001` and `43004000` under round-to-nearest | EAX `42F08001`, AL `01`; still no Boolean return contract |

The S1 f32 store at z+8 rounds a halfway value to even (`0x43004000`); this is a proposed numerical
check with round-to-nearest, not a claim to have measured the game FPU control word in this lane.
The body has no calls or branches, so registration and Init do not select another return path.
Successful Write after S0 puts those position bytes at offsets 44–55 of the normal 57-byte empty-holder
record. Other fields follow their own writer rules (for example, input flags F4 are stored by Read but
written as D4 after `AND DD` at `0x005E8D12`). Position never changes tile/orientation/empty or the holder.

**Actually measured previously**, per LG32/LG33: both `vlfr4-r2-on-reload-2032-20261022a` and
`vlfr4-r2-normal-diagnostic-20261022b` reported S0 effects and 57-byte record agreement with SDK bool false,
SetOrientation(-123) true, same-ID SetTileID(`0x02F10010`) returning that previous ID; V1 with an initially
empty/null-parent holder returned native true / 39 of 57 / error 0 versus Rust false. Their bad outer/base
versions and final-byte truncation rejected on both sides. This document supplies no new game run, no
rescoring of divergent subsequent state, and no update to implementation expected-value files.

## 5. Primary evidence and limits

- Landfill reader: `0x00477560`–`0x004775EE`; the base Boolean is tested at `0x0047758D`.
- Base reader: `0x005E8D60`–`0x005E8DED`; holder call `0x005E8DD8`, immediate stream error call
  `0x005E8DE1`, AL-zero test encoded by the return arithmetic `0x005E8DE4`–`0x005E8DEB`.
- Exemplar reader: `0x005FF420`–`0x005FF501`; plain AL test `0x005FF454`–`0x005FF456`, key zero
  initialisation `0x005FF465`–`0x005FF46D`, key processing precedes `GetError` at `0x005FF4E4`.
- Plain reader: version `0x005FF298`–`0x005FF2AA`, count and clearing `0x005FF2AD`–`0x005FF2C4`,
  error check/return `0x005FF39E`–`0x005FF3B6`.
- Position read helper: `0x005DA550`–`0x005DA57C` makes three consecutive float32 calls without testing
  primitive AL. Bounds helper: `0x004773D0`–`0x00477412` uses f32 constants 8 and 16.
- Segment-stream sticky guards: GetUint8 `0x005D1960`–`0x005D198D`, GetUint16
  `0x005D19C0`–`0x005D19ED`, GetUint32 `0x005D1A20`–`0x005D1A4D`, GetFloat32
  `0x005D1AB0`–`0x005D1ADD`; GetError `0x005D16B0`–`0x005D16BA` returns 0 or 1.
  Atomic short-read expectations use the existing foundations §2.2 contract; this round does not re-prove
  every underlying memory/file-stream transport.
- Position body `0x00477600`–`0x00477657`; EAX loaded with z at `0x0047760F` and retained to return.
  Caller `0x006BC8F4`–`0x006BC8FB` ignores it and calls Init. Literal table pointer at `0x00A8CDC4`
  points to this body; landfill constructor installs its table at `0x0047728A`. GetPosition
  `0x005B2B00`–`0x005B2B15` and GetBoundingBox `0x00477420`–`0x0047744A` copy output words.

All cited instructions were inspected with `re_query.py asm --bytes` from the Windows export of the
original 1.1.641 executable. Bounded read-only PE data inspection independently checked the vtable pointer
and the 8/16 constants. REA was unavailable: `target_unavailable`, “No app is open.” No target was opened,
no code address was invoked, and no native replay or engine source was used. LG31 remains outside scope.
