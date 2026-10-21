# particles: open questions
<!-- Questions about [particles.md](../particles.md). Append new entries at the END; when one is answered, move it to answered/particles.md. Rules: README.md. -->

<!-- from: re-particles4 (2026-10-21) -->
- **PT1 - §2/§3 field meanings (narrowed 2026-10-21, re-particles4: §5.1/§6/§6.1 named `+0x0C`, `+0x1C`, `+0x20`, `+0x28`, `+0x30`, `+0x48`, `+0x50`, `+0x68`..`+0x84` as creation variances, `+0xF0`/`+0xF4`, `+0x160`, `+0x18C`, flags A `0x20`):** still blank: `+0x14`, `+0xA0`/`+0xB8` (and `+0xAC` beyond its first element), `+0xD4`, `+0xD8`, `+0xDC` (a divisor in §5.1 step 8), `+0x100`/`+0x104`, `+0x118`, `+0x120`, `+0x16C`/`+0x16E`/`+0x170`, `+0x184`/`+0x188`, `+0x1B4`, `+0x1C4`, `+0x1DC`..`+0x200`; what the particle factors `P+0x2C`, `P+0x34..+0x40` scale at draw time; and the `force` command's flag bits (`0x10`, `0x4000000`, `0x8000000`, `0x10000000`). Readers: `0x005B9BA0`, `0x005B9790`, `0x005B6600`, the draw functions `0x005B3DA0`/`0x005B4B90`, and the parsers `0x0059C280`, `0x0059A820`.
<!-- from: re-particles4 (2026-10-21) -->
- **PT2 - §2 `+0x1CC`:** what is the reader-B-only list read by `0x005B1D60` (string elements via `0x005811F0`, plus `0x005B1CD0`), and what reads it at run time? Which section version does the shipped `SimCity_1.dat` effect directory carry for the particle section (reader A or B)?
<!-- from: re-particles4 (2026-10-21) -->
- **PT3 - §2 defaults:** the constructor copies the globals `0x00B22D54` (colour `+0x7C`, force `+0xE0`, `+0xF4`, `preferDir` `+0x18C`), `0x00B22D70` (effect particle fields) and `0x00B22E20` as floats. Their file images are pointer-like (`0x00A25EC8`, `0x00A25EE0`, `0x00A261F2`), so they are set by static initialisers. What are their values once the program runs (a runtime read or the initialiser's instructions)?
<!-- from: re-particles4 (2026-10-21) -->
- **PT4 - §5 pools:** the block size and the particle record size of the refill `0x005A61B0` for the two free lists (`0x00B0A6A0`, `0x00B0A688`), and whether either pool is ever freed.
