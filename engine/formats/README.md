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

`fc4-dat` is a small local inspection tool:

```sh
cargo run -p fc4-formats --release --bin fc4-dat -- list <file>
cargo run -p fc4-formats --release --bin fc4-dat -- exemplar <file> 6534284A E7E2C2DB E9590E52 [--resolve]
cargo run -p fc4-formats --release --bin fc4-dat -- ltext <file> <T> <G> <I>
cargo run -p fc4-formats --release --bin fc4-dat -- extract <file> <T> <G> <I> <out> [--raw]
```

## Tests

`cargo test -p fc4-formats` runs unit tests on synthetic fixtures built in the test code (no game data is
committed), including a QFS compress/decompress round-trip property test over 300 pseudo-random inputs, and
truncation and garbage inputs for every parser.

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
