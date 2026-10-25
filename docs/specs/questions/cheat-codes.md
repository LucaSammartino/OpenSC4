# cheat-codes: open questions
<!-- Questions about [cheat-codes.md](../cheat-codes.md). Append new entries at the END; when one is answered, move it to answered/cheat-codes.md. Rules: README.md. -->

<!-- from: re-gzcheats7 (2026-10-24) -->
- **CCM1 - §4:** `GetCheatCodeSaveData` writes the state-ID count as `4·e` (bytes) and `SetCheatCodeSaveData` reads it
  back as a count, inserting `4·e` dwords (the `e` IDs plus `3·e` dwords past the block). Which code saves and
  restores this block (a city or region record?), and what follows it in the buffer, so what the extra IDs are in
  practice? [static: callers of slots 11 and 12 (`+0x2C`, `+0x30`); a reimplementation should insert only the `e`
  saved IDs]
