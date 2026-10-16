<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# fc4-formats

Portable Rust readers for SimCity 4 game data, groundwork for the standalone engine (roadmap M6). No
dependencies, no `unsafe`, no panics on bad input: every parser returns a `Result`. Builds and tests on any OS
and on `i686-pc-windows-msvc`.

| Module | Format |
|---|---|
| `dbpf` | DBPF 1.x packages (`.dat`, `.sc4`, `.sc4lot`, `.sc4desc`, `.sc4model`): header, index 7.0 (20-byte entries; 7.1 24-byte entries only as a flagged standalone extension), the DIR record of compressed entries, random-access reads from a file (`Package::open`) or any byte buffer such as a caller-owned memory map (`Package::from_bytes`). `PackageBuilder` writes DBPF 1.0 packages (used by the tests; foundations §3.1 compression rule available as `Compression::Auto`). |
| `qfs` | QFS/RefPack with the SimCity 4 header (`10 FB` / `11 FB` only): decompressor and a greedy hash-chain compressor |
| `exemplar` | Exemplars and cohorts, binary (`EQZB`/`CQZB`) and text (`EQZT`/`CQZT`), read as the original's factories read them (data-formats §4); raw and converting property getters (§5); `ExemplarStore` loads records with their parent cohorts into `LoadedExemplar`s (own and effective property lists) |
| `ltext` | LTEXT records (UTF-16 and 8-bit forms, 24-bit count, trailing bytes ignored) |
| `fsh` | FSH (`SHPI`) textures, type `0x7AB50E44`, as the factory accepts them (graphics-formats §2): header, directory, entry headers, the attachment chain and its palette search, the code → (format, type) table, the accept rules, the 16-byte-aligned mipmap levels read from the record as §2.8 says (zero only past `size`), frames; `Fsh::decode` turns any level of any frame into RGBA8 (§2.11) |
| `s3d` | S3D (`3DMD`) models, type `0x5AD0E817` (graphics-formats §3): the chunk grammar of each version with every length ignored, legacy and new vertex blocks, the packed and standard vertex-format tables, index, primitive and material blocks, animation with mesh dropping, PROP and REGP; `Model::frame(f)` gives the meshes of a frame and their draw calls; bounding boxes are computed |
| `keys` | graphics-formats §4: the texture lookup order of a material (shared group, the model's group, the fallback `0x07F94200`) and the RKT0–RKT3 model-instance arithmetic with the detail table |

`fc4-dat` is a small local inspection tool:

```sh
cargo run -p fc4-formats --release --bin fc4-dat -- list <file>
cargo run -p fc4-formats --release --bin fc4-dat -- exemplar <file> 6534284A E7E2C2DB E9590E52 [--resolve]
cargo run -p fc4-formats --release --bin fc4-dat -- ltext <file> <T> <G> <I>
cargo run -p fc4-formats --release --bin fc4-dat -- extract <file> <T> <G> <I> <out> [--raw]
cargo run -p fc4-formats --release --bin fc4-dat -- fsh <file> 7AB50E44 1ABE787D <I> [<out.tga>]
cargo run -p fc4-formats --release --bin fc4-dat -- s3d <file> 5AD0E817 BADB57F1 <I>
```

`fsh … <out.tga>` writes frame 0's level 0 as a 32-bit TGA, for looking at a texture locally. Decoded game data
must never be committed.

## Tests

`cargo test -p fc4-formats` runs unit tests on synthetic fixtures built in the test code (no game data is
committed), including a QFS compress/decompress round-trip property test over 300 pseudo-random inputs, and
truncation and garbage inputs for every parser. The FSH, S3D and key tests are named after graphics-formats §8
(`t01_…` to `t41_…`); tests 19 and 20 are rebuilt synthetically there and checked on the shipped records by the
sweep.

`tests/game_data.rs` sweeps the installed game when present: `FC4_GAME_DIR`, or the default GOG path
`C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition`. It opens every `.dat`, `.sc4`, `.sc4lot`,
`.sc4desc` and `.sc4model` file below it (read-only), reads and decompresses every entry, re-compresses a sample,
parses every exemplar, cohort and LTEXT record (exemplars with the factory's type check), loads every `.dat`
exemplar and cohort with its parent chain (a record that does not load is a failure), and prints counts. It
passes with a note when no game data is found. For the statistics:

```sh
cargo test -p fc4-formats --release --test game_data -- --nocapture
```

Result on SimCity 4 Deluxe (GOG), 2026-10-12: 22 DBPF `.dat` files (`unins000.dat` is not DBPF) with 183,205
entries (1.8 GB unpacked, 110,410 compressed), 417 city saves with 49,096 entries; 8,957 binary and 2 text
exemplars and 388 cohorts, all parsed, all binary ones re-encode byte-identically; 77,537 + 1,359 LTEXT
records; every cohort chain resolves (deepest 3 levels); no failures. 2026-10-14, with the data-formats §4 reader:
the same counts, and all 9,347 `.dat` exemplars and cohorts load (every binary record already has its properties
in ascending ID order with no duplicates, so the re-encode is still identical). Two text properties of
`Plugin_002_TheLivingMall.dat` read differently: `LotConfigPropertyWealthTypes` and
`LotConfigPropertyPurposeTypes` are `Uint8:0:{}`, which §4.3 reads as a scalar 0 (`reps` 0 is one value, missing
values are 0), where the old reader made an empty array. The original stores the same scalar 0, and its readers
of these two properties copy `GetCount()` bytes, 0 for a scalar, so the result is the same as an empty array
(data-formats §4.3, EX16).

A second sweep, `sweep_fsh_and_s3d`, loads every FSH and S3D record of the `.dat` files (for each key the first
index entry of its package) and checks the counts of graphics-formats §2.13, §3.14 and §8.4. It decodes every level
of every frame in release builds and every 50th record in debug builds. Result, 2026-10-14: 40,549 FSH records
(36,177 compressed) with 45,387 entries, all accepted; codes `0x61` 25,957, `0x60` 19,357, `0x7D` 40, `0x7B` 24,
`0x7F` 9; 167 DXT1 entries need the §2.8 zero-fill; 48,029 levels decoded. With the GF9 and FS4/FS5 answers
applied (2026-10-14): the same counts; each of the 167 reads its last level from the following `0x70` block inside
the record (no zero byte); 15 frames of multi-frame DXT1 records have an A that alone would give the other DXT1
format, and now report frame 0's (FS5); no later `0x7B` frame lacks a palette. 42,915 S3D records (42,868
compressed; 1.5: 42,713, 1.3: 202), all loaded: 167,070 vertex blocks, 155,661 index blocks (15,311 empty), 154,046
primitive blocks with 138,735 primitives (all triangle lists), 155,827 materials, 155,756 meshes kept and 71
dropped, every draw inside its buffers; all 155,822 non-zero texture IDs resolve at step 1 of §4.1 (2 IDs are 0).
The shipped records of §8.1 tests 19 (unpadded mips) and 20 (the fallback texture) match.

`tests/save_records.rs` is the save sweep's record mode (dev-dependency `fc4-core`). For every distinct `.sc4` below
`FC4_SAVES_DIR` (else the game directory, as above) it finds the history warehouse record (class `0x89EFA536`) and
the 24-hour clock record (class `0xA7E7F929`) of the class-object store (foundations §3.3), checks each entry's size
and CRC (§3.4), reads the payload with the `fc4_core` history and clock cores (to the last byte), writes it back
and compares: a version-2 history payload must be byte-identical; an older one is converted to version 2 (history
§7) and the conversion must be stable (read and written again, unchanged); the clock's 26-byte version-2 output
must equal the saved bytes with the version set to 2 and, below version 2, the 10-byte timer record left out
(clock24 §7). Failures name the save, the object and the first differing byte. `FC4_SAVE_RECORDS_VERBOSE=1` prints
every record. Synthetic records (both versions of each, a wrong CRC, trailing bytes, a cut record) are unit tests in
the same file.

```sh
cargo test -p fc4-formats --release --test save_records -- --nocapture
```

Result on the GOG install, 2026-10-15: 417 saves (417 distinct), each with one history and one clock entry, all
CRCs correct. History: 14 version 2, byte-identical; 403 version 1, converted, stable; 26,552 series, 127,182
points. Clock: 407 version 1 and 10 version 2, all 417 identical after the version normalisation. No failures.

`tests/network_rules.rs` sweeps the shipped network rule resources with `fc4_core::network_rules`
(`docs/specs/network-rules.md` §2): every resource of type `0x0A5BCF4B`, group `0xAA5BCF57` in the install's
top-folder `.dat` files and the network INI (instance `0x8A5993B9`), read at run time and never stored in the
repository (the first package in file-name order wins a key held twice; later copies are reported). Each tile file
must read to its end; it reports the `#Transmogrify` counts (source rules, rules kept, variants dropped as repeats).
Each INI resource reports its sections and entries used and ignored; in RUL0 a `CopyFrom` whose rule is not in the
map yet does nothing (§10.1) and is reported, any other failure fails the test. Then the §2.1 load order
(`NetworkRules::reload`) runs over the same bytes. The bridge and tunnel evaluators (instances `0x1000` and up) are
outside the spec and only reported.

```sh
cargo test -p fc4-formats --release --test network_rules -- --nocapture
```

Result on the GOG install, 2026-10-16: 33 resources, all in `SimCity_1.dat`, no duplicates. The 24 tile files all
read to the end (`Complete`): 5,578 source rules, 6,295 after `#Transmogrify`, 186 variants dropped; four files use
`#Transmogrify` (Avenue 41 → 276 and 6 → 44, LightRail and Monorail 41 → 263 each). Per type (evaluator 1, 2):
Road, Street, Pipe and Subway 326 / 412, Rail 312 / 247, Highway and GroundHighway 36 / 38, Avenue 44 / 276,
LightRail and Monorail 150 / 263, OneWayRoad 334 / 410, DirtRoad 334 / 412, PowerLine none. RUL1: 30 sections, 1,330
entries used (1,324 distinct solutions over 29 type pairs; a repeated key overwrites, §8.1). RUL2: 844 rule
overrides. Network INI (key `00000000-8A5971C5-8A5993B9`): 15 sections, 22 `[TileVerifiers]` entries for 14
pieces, none ignored. RUL0: 251 sections, 249 intersection rules, no parse failures; five `CopyFrom` entries name a rule not in the map at that point (four a rule
that never appears, one the rule itself) and do nothing. The five bridge and tunnel resources (`0x1000`, `0x1001`,
`0x1002`, `0x1006`, `0x100C`) are not tile rule files (they stop at the first line as a bad centre).

The commercial, industrial, civic and park records have the same mode in `fc4-capi`
(`capi/src/save_sweep_tests.rs`, through the C ABI and its test hosts; same `FC4_SAVES_DIR`):
`cargo test -p fc4-capi --release save_sweep -- --nocapture` (see `engine/README.md`, "Save sweep").

## Sources

Written from public format documentation, checked against the shipped data:

- DBPF header, index and hole table, DIR record: [SimsWiki "DBPF"](https://db.modthesims.info/wiki.php?title=DBPF)
- QFS/RefPack header and control codes: [SimsWiki "DBPF Compression"](https://db.modthesims.info/wiki.php?title=DBPF_Compression);
  the variant names in the [`refpack` crate documentation](https://docs.rs/refpack) (documentation only, no code used)
- Exemplar and cohort binary layout and value types: [SimsWiki "EXMP"](https://db.modthesims.info/wiki.php?title=EXMP)
- Exemplar value-type numbering, the original writer's compression rule, the DIR key: `docs/specs/foundations.md`
  §3.1, §9.2, §10
- How the original reads all of these, and where it differs from the public documentation:
  `docs/specs/data-formats.md` (§11 lists the differences)
- LTEXT layout and the text-exemplar syntax: common modding knowledge (SC4Devotion wiki, which refused automated
  access), then confirmed and corrected on the shipped data as described below.
- FSH, S3D and the model and texture keys: `docs/specs/graphics-formats.md` only (§2–§4; its §9 lists where the
  common modding descriptions differ, summarised below).

## Findings and choices

Where the documentation was silent or wrong, the code decides and tags the decision `[choice]`:

- **QFS long copy code** (`0xC0..=0xDF`): the "DBPF Compression" page also lists a "SimCity 4" variant with a
  different bit layout. The standard RefPack layout (length `((b0 & 0x0C) << 6) + b3 + 5`, distance
  `((b0 & 0x10) << 12) + (b1 << 8) + b2 + 1`) decodes all 132,438 compressed entries of the install to exactly
  the size in both the QFS header and the DIR, so that is what we use.
- **The DIR is authoritative** for compression. One city save holds a 34-byte uncompressed class record whose
  bytes 4–5 happen to read `11 FB`, a valid-looking RefPack signature. It is not listed in the DIR and is read
  as stored. A listed entry that fails the QFS acceptance check fails the read, as in the original (data-formats
  §2.6; none occur in the install).
- **Text exemplar parent key order is group, instance, type**, not the binary type, group, instance. Both text
  exemplars in the install end their `ParentCohort` key with the cohort type `0x05342861`, and only in that order
  do their parents resolve. Resource keys inside property values keep type, group, instance order.
- **The LTEXT type `0x2026960B` is shared**: `Sound.dat` stores 1,587 XA and 21 RIFF audio clips and 72 bare text
  files under it, and one locale entry is plain text without a header. `ltext::is_ltext` tells real records apart.
- **LTEXT has an 8-bit form**: encoding byte `0x00` followed by `count` bytes (each locale file has one; city
  saves use it for names such as `Temp.png`). Besides the documented UTF-16 form (encoding byte `0x10`). The
  count is 24 bits and bytes after the string are ignored (data-formats §6.1).
- **Exemplars follow data-formats §4** (the public "value type, key type, unused byte" description is the same
  bytes): only the first 4 signature bytes are checked; each binary property is a kind byte (0 value, 1 resource
  key) and a `u32` foundations §9.2 disk tag, so the "unused" and key-type high bytes must be 0; tag `0x000C` is
  one RZChar byte, not a string; Empty, void, Char and RZUnicodeChar arrays and the Uint64 / Sint8 / Sint16 /
  Float64 types are read. A parent key that does not load as a cohort, or a kind-1 resource that does not load,
  fails the whole record. A kind-1 resource of another type loads only when a resource cache holds it
  (`ExemplarStore::add_resource`) or a factory registered for its type (`add_factory`) reads it from a loaded
  package; an index entry alone is not enough (§4.6). `[choice]`: cycles and chains deeper than 32 fail the load
  (the original recurses); a record that failed only on depth is loaded again when reached shallower.
- Text exemplars follow §4.3: the 2 bytes after the signature are skipped unchecked, numbers use rule N (`10` is
  decimal, `1A` hexadecimal) on the raw text, type names are case-sensitive and not trimmed, exactly `PropCount`
  lines are read. `ParentCohort` fields are not counted (a missing one is 0, nothing inside the braces is trimmed);
  `reps` and a scalar's value are not trimmed, an array's values are; `reps` values are taken with missing ones 0
  and no cap in the original (`[choice]` the read fails above 16 MiB of values); 64-bit values are hexadecimal
  without a sign; only a string ends at the first `}`. Float32 values go through the game's own decimal parser at
  the main thread's 24-bit x87 precision (`exemplar::parse_float`), so `0.05` is `0x3D4CCCCC` and `1e10`
  1,410,065,408 (10ⁿ wraps in 32 bits). The text writer emits Float32 text that this parser reads back exactly.
- Getters follow §5: `as_u32` / `u32s` / `count` ... are the raw accessors (`[choice]` a type mismatch is a missing
  value, where the original reinterprets the bytes), `get_u32` / `get_f32` / `get_string_into` ... convert: a
  scalar through int64 (floats truncated by `_ftol`, wrapping; Char and RZChar sign-extended) or float64 (Uint32 from
  2³¹ and every Uint64 keep 24 significant bits); arrays, strings and Empty through the text round trip (the first
  element of a numeric array, floats in fixed point and not exact on the way back, a void array as the empty text).
  Empty reads as 0 and succeeds; the string getter writes any value's text and returns true only for a string;
  `[choice]` an object (kind-1) value is a missing property. `get_void` is holder slot 5: a raw copy from a void
  array only.
- Duplicates: `Package::find` returns the **first** index entry with a key, and the first DIR record wins
  (data-formats §2.3, §2.5); inside an exemplar the last property with an ID wins (§4.5), and both property lists
  are sorted by unsigned ID; `ExemplarStore` lets a later insert replace an earlier one (load order).
- Floats as text (the getters' round trip, data-formats §5.1, EX29): fixed point, the first 17 significant digits
  rounded half-up, trailing fraction zeros dropped, cut at 63 characters (a Float32 too); −0.0 prints `0` (EX24) and
  a signalling NaN prints `1.#QNAN` (EX28). In text exemplars a Bool is trimmed of leading spaces and tabs (EX26), a
  String of spaces and tabs at both ends before its quotes are removed (EX27).
- `ExemplarStore` reads records eagerly, under the conditions of data-formats §3 (EX31): only the first entry with a
  key in a package is read, a later package's record wins even when it fails to read (the key then fails with
  `LoadError::Unreadable`, no fallback), and `add_factory` refuses a type that a loaded package already holds, so
  register factories before loading packages.
- Index entries are 20 bytes and the index minor version is ignored (data-formats §2.1). `[choice]`: when the
  index size is exactly `24 × count` the entries are read as index 7.1, with a warning (the original cannot open
  such a package). The DIR record is always 16 bytes per key.
- Open checks follow data-formats §2.1–§2.4: major version ≤ 1, index and hole tables inside the file, and
  **every** entry with `offset ≥ 0x60` and `offset + size ≤ file length`; one bad entry fails the whole open.

## FSH and S3D: decoding choices

The original never decodes 3D textures on the CPU; the renderer does. FreeCity4 decodes to RGBA8 as
graphics-formats §2.11 chooses (`[choice]`, Q17):

- **DXT1** (`0x60`): 5:6:5 colours expanded by bit replication; c0 > c1 (as `u16`) gives the 4-colour block,
  otherwise c2 = (c0 + c1) / 2 and c3 is **transparent black whatever the 12-bit field A says**. A only picks the
  reported format (7 DXT1-RGB or 8 DXT1-RGBA, Q5); DX7 makes both the one Direct3D DXT1 surface, which decodes
  index 3 as (0, 0, 0, 0), so this matches (GF3). Divisions on the 8-bit values, rounded down (FS8: the GPU does it
  in the original).
- **DXT3** (`0x61`): 4-bit alpha × 17, pixel 0 in the low nibble; colour block always 4-colour. **DXT5** (`0x62`):
  the usual 8-value alpha ramp (6 or 4 interpolated values, rounded down); colour block always 4-colour.
- **Uncompressed**: `0x7D` B, G, R, A; `0x7F` B, G, R (alpha 255); the 16-bit codes in their Direct3D layouts,
  `0x6D` A4R4G4B4 × 17, `0x7E` A1R5G5B5, `0x78` R5G6B5 (no renderer of the original shows them correctly, GF2);
  `0x64` is (255, 255, 255, a), `0x65` (L, L, L, a), both read unsigned although the original asks for signed bytes
  (Q6). DX7 cannot create `0x64 0x65 0x6D 0x78 0x7B 0x7E` at all (`fsh::dx7_creatable`) and draws a model using one
  untextured, in its diffuse colour (GF1); FreeCity4 decodes and shows them, as the original's OpenGL renderer does
  for all but `0x78`.
- **`0x7B`** (8-bit index): the index goes **through the palette** (`0x24` B, G, R; `0x2A` B, G, R, A; `0x2D`
  A1R5G5B5; `0x29` R5G6B5), where the original uploads the indices as luminance; the shipped palettes are identity
  grey ramps, so the result is the same (Q7). `Fsh::level_data` gives the raw indices, which the original's terrain
  code reads on the CPU (GF4). Only **frame 0** must have a 256 × 1 palette with its 256 entries inside the record,
  else the record is rejected (the original checks frame 0 only and reads uninitialised data when there is none,
  FS4). A later frame uses its own palette when it is usable; otherwise it decodes as grey, (i, i, i, 255), the
  original's luminance upload read unsigned.
- **One (format, type) for every frame** (FS5): the pair is decided from frame 0's code and A; a later DXT1
  frame's own A is never read. The pixels are the same either way.
- **Unpadded mips** (Q4, GF9): levels are read at the 16-byte-aligned offsets the game uses, **from the record**:
  in the 167 shipped entries the last level reads the first 8 bytes of the following `0x70` name block, as in the
  game. Only bytes past `size` read as zero (none shipped). An entry's own bytes end at its first attached block,
  another entry's header or `size`, whichever comes first after its pixel start (FS1: the original has no entry
  end; this end only decides acceptance). Levels that would not fit even packed back to back are rejected; when only
  the aligned layout overruns, the entry is the §2.8 mip-padding case (FS2, `Frame::zero_filled`, a name that
  predates GF9).
- A DXT level whose width is not a multiple of 4 decodes whole blocks read from the level's start, so the missing
  bytes are the record's following bytes, as DX7 copies them (FS7; zero only past `size`), and is cropped. A DXT
  texture is at least 4 × 4 (§2.7), so a 2 × 2 DXT entry decodes to 4 × 4.

Other `[choice]` checks of the FSH reader (the original checks no offset, Q2): `size` must cover the header and
directory, the record must hold `size` bytes (bytes after it are ignored), a record needs at least one entry (the
original also fails a zero count, FS3), and every entry header, attachment header and frame 0's palette must be
inside `size`. A negative (sign-extended) height fails the power-of-two test; a negative width counts as below the
minimum (both as in the original, FS6).

The S3D reader rejects, `[choice]`: an unknown vertex format (stride 0, Q11), a format whose stride is below 12
(the bounding box reads 12 bytes per vertex, FS9), a legacy block whose stored stride is below the format's standard
stride (the draw would read past it, FS10), an index size other than 2 (Q12) and more than 2 textures (Q13); the
original loads all of these and misreads. It accepts a vertex block with no vertex, with the empty box (min 1,
max 0). Element offsets use the plain kind-order layout (§3.5). A name without a NUL is its whole stored bytes;
names are UTF-8 when valid, else Latin-1; REGP names are lower-cased, ASCII letters always and, in a valid UTF-8
name, every letter by Unicode simple case mapping (FS12; the original uses `towlower` in the game's locale; all
shipped names are ASCII). PROP entries are sorted stably by frame, signed and ascending (FS11; the original's
`std::sort` keeps equal frames in file order only up to 16 entries; every shipped entry is frame 0). A material
texture's filters (0, 0) are nearest without mipmaps: no render property overrides them in normal play (GF5). The
vertex colour element is B, G, R, A (GF6). `Mesh::draw_calls` does not clip a primitive whose range leaves its
buffers (it reports the index slice as `None`); `Mesh::ranges_fit` says whether every draw fits.

`keys::resolve_texture` returns no texture (`None`) when neither texture key nor the fallback `0x07F94200` loads:
the original leaves such a slot unbound with its ID set to 0 (FS13). `keys::model_key` returns no model for a zoom
above 4 or too few values, clamps the detail to 0–2 and takes the rotation mod 4 (FS14). The detail is the
original's `kModelLODOffset` render property (22) clamped to 0–2, set from Texture Quality: High 0, Medium 1, Low 2
(GF8).

## Community descriptions versus the exe

graphics-formats §9: where the common modding descriptions of FSH and S3D differ from what the exe does, the code
follows the exe.

| Topic | Commonly described | What the reader does |
|---|---|---|
| FSH entry `+0x0C`/`+0x0E` | x and y "position", mip count in the top 4 bits of `+0x0E` | one dword: mip count in bits 28–31 (same bits), the low 12 bits the DXT1 RGB/RGBA selector A; nothing else is read |
| FSH codes | `0x6D` 4444, `0x78` 565, `0x7E` 1555, `0x7B` 8-bit, compressed-bitmap flag `0x80` | the factory accepts these and `0x64`, `0x65`; DX7 creates only DXT, 8888 and 888; a code with bit `0x80` is rejected, not decompressed |
| FSH palettes | several palette codes, including `0x22` | only `0x2A`, `0x24`, `0x2D`, `0x29`, searched only after `0x7B`/`0x7A` |
| FSH directory ID, centre fields | meaningful | ignored (kept for inspection) |
| FSH mipmaps | levels follow each other | each level 16-byte aligned from the pixel start; DXT levels stop shrinking at 4 × 4 |
| S3D lengths | chunk sizes | ignored (unreliable in the shipped data) |
| S3D VERT | "1.3 and earlier: `u16` format + `u16` stride" | true for 1.0–1.3; 0.x uses the new layout; the draw stride is always the format's standard one |
| S3D INDX stride | any | must be 2 |
| S3D materials | class, reserved, anim rate/mode, texture name are attributes | read and ignored; flags keep 16 bits OR `0x180`; at most 2 textures; a last texture ID of 0 clears the count |
| S3D frame index −1 | "hidden in this frame" | any negative or out-of-range index drops the whole mesh at load |
| Bounding box | sometimes described as stored | computed from the positions |
