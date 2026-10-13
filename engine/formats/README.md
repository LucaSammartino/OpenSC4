<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# fc4-formats

Portable Rust readers for SimCity 4 game data, groundwork for the standalone engine (roadmap M6). No
dependencies, no `unsafe`, no panics on bad input: every parser returns a `Result`. Builds and tests on any OS
and on `i686-pc-windows-msvc`.

| Module | Format |
|---|---|
| `dbpf` | DBPF 1.x packages (`.dat`, `.sc4`, `.sc4lot`, `.sc4desc`, `.sc4model`): header, index 7.0 (20-byte entries; 7.1 24-byte entries only as a flagged standalone extension), the DIR record of compressed entries, random-access reads from a file (`Package::open`) or any byte buffer such as a caller-owned memory map (`Package::from_bytes`). `PackageBuilder` writes DBPF 1.0 packages (used by the tests; foundations §3.1 compression rule available as `Compression::Auto`). |
| `qfs` | QFS/RefPack with the SimCity 4 header (`10 FB` / `11 FB` only): decompressor and a greedy hash-chain compressor |
| `exemplar` | Exemplars and cohorts, binary (`EQZB1###`/`CQZB1###`) and text (`EQZT1###`/`CQZT1###`); typed property API; `ExemplarStore` with cohort inheritance |
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
parses every exemplar, cohort and LTEXT record, resolves every exemplar's cohort chain, and prints counts. It
passes with a note when no game data is found. For the statistics:

```sh
cargo test -p fc4-formats --release --test game_data -- --nocapture
```

Result on SimCity 4 Deluxe (GOG), 2026-10-12: 22 DBPF `.dat` files (`unins000.dat` is not DBPF) with 183,205
entries (1.8 GB unpacked, 110,410 compressed), 417 city saves with 49,096 entries; 8,957 binary and 2 text
exemplars and 388 cohorts, all parsed, all binary ones re-encode byte-identically; 77,537 + 1,359 LTEXT
records; every cohort chain resolves (deepest 3 levels); no failures.

## Sources

Written from public format documentation, checked against the shipped data:

- DBPF header, index and hole table, DIR record: [SimsWiki "DBPF"](https://db.modthesims.info/wiki.php?title=DBPF)
- QFS/RefPack header and control codes: [SimsWiki "DBPF Compression"](https://db.modthesims.info/wiki.php?title=DBPF_Compression);
  the variant names in the [`refpack` crate documentation](https://docs.rs/refpack) (documentation only, no code used)
- Exemplar and cohort binary layout and value types: [SimsWiki "EXMP"](https://db.modthesims.info/wiki.php?title=EXMP)
- Exemplar value-type numbering, the original writer's compression rule, the DIR key: `docs/specs/foundations.md`
  §3.1, §9.2, §10
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
- Binary exemplar key type `0x00` is a single value (the following byte is unused), `0x80` an array with a `u32`
  count; strings are always arrays whose count is the byte length. Text exemplars write `reps` 0 for single
  values (strings included) and allow an empty `{}` value.
- The value-type table also accepts the codes of foundations §9.2 that the exemplar page does not list
  (Uint64 `0x0400`, Sint8 `0x0500`, Sint16 `0x0600`, Float64 `0x0A00`); none occur in the install.
- Duplicates: `Package::find` returns the **first** index entry with a key, and the first DIR record wins
  (data-formats §2.3, §2.5); exemplar lookups return the last property with an ID; `ExemplarStore` lets a later
  insert replace an earlier one (load order).
- Index entries are 20 bytes and the index minor version is ignored (data-formats §2.1). `[choice]`: when the
  index size is exactly `24 × count` the entries are read as index 7.1, with a warning (the original cannot open
  such a package). The DIR record is always 16 bytes per key.
- Open checks follow data-formats §2.1–§2.4: major version ≤ 1, index and hole tables inside the file, and
  **every** entry with `offset ≥ 0x60` and `offset + size ≤ file length`; one bad entry fails the whole open.
