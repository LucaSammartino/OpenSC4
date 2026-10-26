# Save-stream helpers: the shared `operator<<` / `operator>>` (behaviour spec)

Scope: the owners `fn:operator<<` (17 functions, 2,159 bytes) and `fn:operator>>` (28 functions, 3,680 bytes) as
tools/coverage.py assigns them. Most are the small stream helpers that many save records share to write or read a
value type (a vector, a box, a timer, a generator, a grid); the record specs name them where they are used. Two
are not stream helpers at all: the particle description writer (§5) and the automata attractor's destructor (§6),
which FreeCity4's symbol names also call `operator<<`.

Related: foundations.md §2 (the stream model, the primitive encodings, the sticky error), output-streams.md §1 (the
stream vtable slots), lot-records.md §2.1 (the short names `bool`, `vec3`, `box`, `day`, `bytes`, `xform` that
the record tables use), simgrid.md §7.4 (the scanning iterator), weather.md §10 (the generator, the iterator and
the byte grids), particles.md §2 (the particle description), automata.md §3.5 (the attractor record).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of both owners was read in the decompile, and the readers that keep a
count or a flag in a temporary were read in the disassembly. Written 2026-10-24 (re-streamops7).

## 1. Conventions *[compat]*

- Every helper takes the stream and the value and returns the stream; none checks the stream's error itself.
  Callers test `GetError` once at the end of their record (foundations §2.9).
- The stream slots used: 4 `Sint8`, 5 `Uint8`, 7 `Uint16`, 8 `Sint32`, 9 `Uint32`, 11 `Uint64`, 12 `Float32`,
  13 `Float64`, 14 `SetRZCharStr` (output-streams.md §1.3; the input stream uses the same numbers for the `Get`
  methods).
- Unless noted, a reader reads straight into the destination fields, so a failed read leaves each field it did not
  reach unchanged (reads are atomic, foundations §2.2).

## 2. Value types *[compat]*

| Name | Write | Read | Bytes | Encoding | Used by (examples) |
|---|---|---|---|---|---|
| `bool` | `0x005D9FD0` | `0x005D9FA0` | 1 | u8 0 / 1; read as byte ≠ 0 | lot-records.md §2.1 |
| `key3` | `0x005DA0A0` | `0x005DA070` | 12 | 3 × u32 type, group, instance: the bytes of `WriteResKey` (foundations §2.5) | automaton model keys (automata.md §6.1), MySims (`0x00425490`), the MySim dispatch |
| `day` | `0x005DA1A0` | `0x005DA170` | 4 | u32 day number of a `cRZDate` (lot-records.md §2.1) | lot records |
| `timer` | `0x005DA240` | `0x005DA1C0` | 10 | §2.1 | MySims, the riot disaster (write `0x0057D833`, read `0x0057E8AD`), the 24-hour clock's version-1 record (clock24.md §7), the simulator (`0x007023F2`) |
| `rng` | `0x005DA2E0` | `0x005DA2A0` | 20 | §2.2 | flora.md §10.1 field 16, the weather generator (weather.md §10) |
| `scanner` | `0x005DA480` | `0x005DA410` | 29 | s8 state, s32 x, s32 z, u32 step, s32 x0, z0, x1, z1 (simgrid.md §7.4; the state byte is stored as read) | prop developer (prop-developer.md §7 field 3), traffic simulator, plumbing, residential simulator |
| `vec2` | `0x005DA520` | `0x005DA4F0` | 8 | 2 × f32 | the particle description (particles.md §2), the brush description (effects.md §8.1) |
| `vec3` | `0x005DA580` | `0x005DA550` | 12 | 3 × f32 x, y, z | lot records, automata |
| `box` | `0x005DA600` | `0x005DA5B0` | 24 | 6 × f32: min x, y, z, max x, y, z | the particle description, the construction occupant (`0x005CE770`), lot records |
| `mat3` | `0x005DA680` | `0x005DA650` | 36 | 9 × f32 in memory order (3 rows of 3) | the transform below |
| `mat4` | `0x005DA6E0` | `0x005DA6B0` | 64 | 16 × f32 in memory order (4 rows of 4) | network occupants (network-occupants.md) |
| `xform` | `0x005DA8C0` | `0x005DA710` | 1 + … | u8 mask, then `mat3` if mask & 4, 3 × f32 translation if mask & 1, f32 scale if mask & 2 (lot-records.md §2.1; the read rules are network-occupants.md NO15) | lot records, network occupants |

### 2.1 The timer *[compat]*

A `cRZTimer` (clock24.md §4.3) is written as: u8 the timer's units (the low byte of its first field), u64 its
elapsed time (`GetElapsedTime`), u8 1 when it is running (`IsTimerRunning`), else 0.

The reader first resets the timer, then reads the u8 units and sets them, reads the u64 and sets the elapsed time,
reads the u8 running flag and starts the timer when it is non-zero, otherwise stops it. The units and the flag are
read into temporaries that are not initialised: on a failed stream the timer is reset and then given whatever the
temporaries held *[quirk]*. *[choice]* FreeCity4 leaves the timer reset and stopped when the read fails.

### 2.2 The random generator *[compat]*

The generator record is: u32 the integer-path state (`+0`), u32 the double-path state (`+4`), s32 the
"has a cached Gaussian" flag (`+8`, written with `SetSint32`), f64 the cached Gaussian (`+0x10`). Its meaning is in
weather.md §10 ("Generator").

The reader reads the two u32 and the f64 straight into the generator, but the s32 flag through the stack slot that
held the stream argument: on a failed stream the flag becomes the stream object's address (non-zero, so "has a
cached Gaussian") *[quirk]*. *[choice]* FreeCity4 leaves the flag unchanged when the read fails.

## 3. Containers *[compat]*

| Name | Write | Read | Encoding |
|---|---|---|---|
| random cell iterator | `0x005DAB70` | `0x005DB280` | weather.md §10 ("Iterator") and simgrid.md §7.4 |
| occupant list | `0x005DAFF0` | `0x005DB3C0` | u32 count, then each as a `ref` (foundations §2.7); the reader resolves each with IID `0x26BB767E` and appends it (civic-buildings.md) |
| `bytes` | `0x005DADA0` | `0x005DB520` | u32 n, n bytes (lot-records.md §2.1) |
| response curve | `0x005DB190` (automata-managers.md) | `0x005DB5D0` | §3.1 |
| byte grid | the record's own writer (weather.md §10) | `0x005DB9B0` | §3.2 |
| bool grid | | `0x005DBAE0` | §3.2 |

### 3.1 Response curve (read) *[compat]*

u32 n, then n × (f32 x, f32 y). The reader fills a temporary point list of n points and hands it to the curve's
`SetResponseCurveData` (`0x005F4480`), which discards the old segments and builds new ones from the points as in
foundations.md §4.2; the temporary is freed. There is no upper bound on n. The one user is the automata
attractor's record (automata.md §3.5, Read `0x00518270`).

The count is read into the stack slot that held the stream argument *[quirk]*: when the stream has already
failed, the count is the stream object's address and the temporary is sized to that many 8-byte points (a huge
allocation). *[choice]* FreeCity4 fails the read and leaves the curve unchanged.

### 3.2 Byte and bool grids (read) *[compat]*

u32 rows, u32 cols, then rows × cols bytes, row by row. When either count is ≥ 1,000,000 nothing more is read and
the grid is left unchanged. Otherwise the grid is resized to `rows` rows (rows added are empty, rows beyond are
dropped), each row is resized to `cols` cells (new cells 0 / false), and each cell is read with `GetUint8`. The
bool grid (a vector of `vector<bool>` rows) stores each byte as true when it is non-zero.

- Users: the weather reads its temperature, moisture and moisture-base grids, and its two legacy grids, as byte
  grids (`0x0072903A`, `0x007290F0`, `0x007290FD`, `0x00729264`, `0x00729285`), and its sea mask as a bool grid
  (`0x00729044`), so any non-zero byte of the sea mask loads as 1.
- The two counts are read into temporaries that are not initialised; a failed stream leaves them holding
  whatever the stack held *[quirk]*. *[choice]* FreeCity4 skips the grid when either count read fails.

### 3.3 The bool-grid container code

The bool-grid reader uses the standard library's vector of `vector<bool>` (element 20 bytes). These functions are
that library code, with no game behaviour of their own: `0x005DAEF0` (`vector<bool>` assignment), `0x005DB040`
(fill a range), `0x005DB070` (copy forward), `0x005DB0C0` (copy backward), `0x005DB220` (uninitialised copy),
`0x005DB250` (uninitialised fill of n), `0x005DB590` (destroy the elements and free the storage), `0x005DB690`
(erase to the end), `0x005DB6F0` (grow and insert: the new capacity is size + max(size, n)), `0x005DB800`
(insert n copies), `0x005DB940` (resize).

## 4. Writing the scanner and the generator

The writers mirror §2: the scanner writer writes the state with `SetSint8`, x and z with `SetSint32`, step with
`SetUint32` and the rectangle with `SetSint32`; the generator writer writes `+0` and `+4` with `SetUint32`, `+8`
with `SetSint32` and `+0x10` with `SetFloat64`.

## 5. The particle description writer (`0x005AFFF0`) *[compat]*

Called by `cSC4EffectsResource`'s writer for each particle description (`0x005A4008`), after it has written the
section's u16 version **2** and the u32 count (`0x005A3FD5`–`0x005A3FEC`). It writes the fields of particles.md §2
in **reader B's order and layout**, matching that version (reader B serves every version other than 1):

1. u32 `+0x00` (flags A), u32 `+0x04` (flags C, written whole), u32 `+0x08` (flags B, written whole).
2. `vec2` `+0x0C`; f32 `+0x14`; u32 `+0x18`; f32 `+0x1C`; `vec2` `+0x20`, `+0x28`; `box` `+0x30`; `vec2` `+0x48`;
   `box` `+0x50`; 5 × f32 `+0x68`…`+0x78`; `vec3` `+0x7C`.
3. `f32[]` `+0x88` (`0x005DAD50`), `vec3[]` `+0x94` (`0x005A3CC0`), `f32[]` `+0xA0`, `+0xAC`, `+0xB8`, `+0xC4`.
4. u32 `+0xD0`; u8 `+0xD4`, `+0xD5`; f32 `+0xD8`, `+0xDC`; `vec3` `+0xE0`; f32 `+0xEC`, `+0xF0`; `vec3` `+0xF4`;
   f32 `+0x100`, `+0x104`; the tractor points `+0x108` (`0x005AFC10`); f32 `+0x114`; `vec2` `+0x118`; `vec3`
   `+0x120`; `f32[]` `+0x12C`.
5. 8 × f32 `+0x138`…`+0x154`; `vec2` `+0x158`; the name `+0x160` with `SetRZCharStr` (its character pointer); u16
   `+0x16C`, `+0x16E`; f32 `+0x170`; `vec2` `+0x174`, `+0x17C`; f32 `+0x184`, `+0x188`; `vec3` `+0x18C`; 3 × f32
   `+0x198`…`+0x1A0`; `f32[]` `+0x1A4`; f32 `+0x1B0`; u32 `+0x1B4`; the wiggles `+0x1B8` (`0x005AFC90`); f32
   `+0x1C4`.
6. The age-event list `+0x1CC` (`0x005AFD00`); f32 `+0x1D8`, `+0x1DC`; `u32[]` `+0x1E0` (`0x005DAEA0`); f32
   `+0x1EC`, `+0x1F0`; `f32[]` `+0x1F4`; f32 `+0x200`.

Reader B masks flags C with `0xFF` and flags B with `0x7FF` while the writer writes them whole, so unknown bits
are dropped on a load and never reach the next save.

## 6. The automata attractor's destructor (`0x005192E0`)

Despite its symbol name, `0x005192E0` is the destructor body of the automata attractor (class `0x6A1628BA`,
automata.md §3.5). The attractor's deleting destructor (`0x005192C0`, called through its vtable) calls its own
slot 4, then this body, then frees the object (`0x005E5620`). The body frees the response curve's point storage
(`+0x5C`), restores the `cSC4AutomataController` vtables, releases the object at `+0x4C`, destroys the list at
`+0x44` (every node, then the sentinel, `0x005650A0`), destroys the `cRZString` at `+0x28` (its storage `+0x2C`)
and releases the object at `+0x0C`.

## 7. Function inventory

`fn:operator<<`: `0x005192E0`, `0x005AFFF0`, `0x005D9FD0`, `0x005DA0A0`, `0x005DA1A0`, `0x005DA240`, `0x005DA2E0`,
`0x005DA480`, `0x005DA520`, `0x005DA580`, `0x005DA600`, `0x005DA650` (named `FUN_`; the `mat3` reader),
`0x005DA680`, `0x005DA6E0`, `0x005DA8C0`, `0x005DAB70`, `0x005DAFF0`.

`fn:operator>>`: `0x005D9FA0`, `0x005DA070`, `0x005DA170`, `0x005DA1C0`, `0x005DA2A0`, `0x005DA410`, `0x005DA4F0`,
`0x005DA550`, `0x005DA5B0`, `0x005DA6B0`, `0x005DA710`, `0x005DAEF0`, `0x005DB040`, `0x005DB070`, `0x005DB0C0`,
`0x005DB220`, `0x005DB250`, `0x005DB280`, `0x005DB3C0`, `0x005DB520`, `0x005DB590`, `0x005DB5D0`, `0x005DB690`,
`0x005DB6F0`, `0x005DB800`, `0x005DB940`, `0x005DB9B0`, `0x005DBAE0`.
