# particles: open questions
<!-- Questions about [particles.md](../particles.md). Append new entries at the END; when one is answered, move it to answered/particles.md. Rules: README.md. -->

<!-- from: re-particles4 (2026-10-21) -->
- **PT1 - §2/§3 field meanings:** name every description field with a blank meaning in §2 (`+0x14`, `+0x28`, `+0x48`, `+0x68`/`+0x6C`/`+0x78`, `+0xA0`/`+0xAC`/`+0xB8`, `+0xD4`, `+0xD8`/`+0xDC`, `+0xF0`, `+0xF4`, `+0x100`/`+0x104`, `+0x118`, `+0x120`, `+0x16C`/`+0x16E`/`+0x170`, `+0x184`/`+0x188`, `+0x1B4`, `+0x1C4`, `+0x1DC`..`+0x200`, `+0x160`) and the `force` command's flag bits (`0x10`, `0x4000000`, `0x8000000`, `0x10000000`) from the runtime readers (`InitEffectsParticle` `0x005B7580`, `0x005B9BA0`, `0x005B9790`, `0x005B6600`) and the `cParticleEmitCommand`/`cParticleForceCommand` parsers (`0x0059C280`, `0x0059A820`).
<!-- from: re-particles4 (2026-10-21) -->
- **PT2 - §2 `+0x1CC`:** what is the reader-B-only list read by `0x005B1D60` (string elements via `0x005811F0`, plus `0x005B1CD0`), and what reads it at run time? Which section version does the shipped `SimCity_1.dat` effect directory carry for the particle section (reader A or B)?
<!-- from: re-particles4 (2026-10-21) -->
- **PT3 - §2 defaults:** the constructor copies the globals `0x00B22D54` (colour `+0x7C`, force `+0xE0`, `+0xF4`, `preferDir` `+0x18C`), `0x00B22D70` (effect particle fields) and `0x00B22E20` as floats. Their file images are pointer-like (`0x00A25EC8`, `0x00A25EE0`, `0x00A261F2`), so they are set by static initialisers. What are their values once the program runs (a runtime read or the initialiser's instructions)?
<!-- from: re-particles4 (2026-10-21) -->
- **PT4 - §5 pools:** the block size and the particle record size of the refill `0x005A61B0` for the two free lists (`0x00B0A6A0`, `0x00B0A688`), and whether either pool is ever freed.
<!-- from: re-particles4 (2026-10-21) -->
- **PT5 - §5 `InitEffectsParticle` `0x005B7580`:** the particle's fields after creation (position from the emission volume `+0x50` box and flags A `0x1000`/`0x2000`, life from `+0x0C`, the random draws and their order).
<!-- from: re-particles4 (2026-10-21) -->
- **PT6 - §6 rest of `Start` `0x005B3000`:** after `preferSea`: the loop-interval countdown `+0x190`, the emission volume, the preroll byte `+0x118`, and the order of every `RandomRange` draw (which generator `0x00913761` uses).
<!-- from: re-particles4 (2026-10-21) -->
- **PT7 - §8 emitter flag bits:** the three emitters of `0x005B96E0` are keyed by "the description's flag bits" `0x04`/`0x02` (effects.md §7.1); which description word holds them (flags A `+0x00` would make `0x04` = `maintain`, `0x02` = `inject`)? Also confirm the geometry-sourced emitter `0x005AA5D0` self-stops as effects.md §7.1 says; §7.1 item 2's class attribution should be corrected per particles.md §8.
