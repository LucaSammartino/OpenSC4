# hit-music: open questions
<!-- Questions about [hit-music.md](../hit-music.md). Append new entries at the END; when one is answered, move it to answered/hit-music.md. Rules: README.md. -->

<!-- from: re-hitman (2026-10-22) -->
- **HM1 - section 6.3 step 1:** what the control group's gate (group slot 4, then slot 20 of the result) tests, and
  whether SC4 ever creates a control group (no caller of `AddToControlGroup` outside Hit was read). [a mute switch;
  no SC4 groups]

<!-- from: re-hitman (2026-10-22) -->
- **HM2 - section 6.3, 6.4:** where a starting track first registers its tick with the sound system (only the
  `Unpause` path, `0x0093B08B`, calls sound-system slot 30), and the unit of its period `0x14` and of the tick's
  `dt`. [registered through the +0x0C interface's slot 18 on start; milliseconds]

<!-- from: re-hitman (2026-10-22) -->
- **HM3 - section 6.3 step 2:** what slots 7, 18 and 19 of the track player's `+0x0C` table are, and who clears the
  byte `+0x5C` that the resume path sets (the tick does nothing while it is set). [playing, unpause, paused; cleared
  by the next normal start]

<!-- from: re-hitman (2026-10-22) -->
- **HM4 - section 5.2:** the roles of TKDT fields 6 (`0xFF706616`), 14 (`0xFFD7EE4D`) and 16 (`0xFF2D7CD1`): read and
  written by the TKDT codec, copied by the sems (field 6), no other reader in the Hit classes. [read by the SC4 audio
  managers]

<!-- from: re-hitman (2026-10-22) -->
- **HM5 - section 3.9:** what `cIHitMan` slots 6 (calls `DuckMapRemoveAll`), 7 (returns true) and 8 (`IsPaused`, the
  byte `+0x21` that no HitMan method writes) are for. [unused pause API]

<!-- from: re-hitman (2026-10-22) -->
- **HM6 - section 3.8:** what the default track-flow error handler `0x009377DD` does: it calls slot 3 of the track,
  slot 9 of that result, then slot 16 of the track. [kills the failing track]

<!-- from: re-hitman (2026-10-22) -->
- **HM7 - section 2, 3.2:** which `.hit` files the shipped Hit INI lists under `TrackDataBlocks` (the INI path comes
  from sound-system slot 20), and the layout of a `.hit` file beyond the `ENTP`…`EENT` entry table. [the INI and the
  .hit data are in the sound packages; layout unknown]

<!-- from: re-hitman (2026-10-22) -->
- **HM8 - section 6.7:** whether the register-set slot 6 used by `UpdatePitch` means "set in the local set" (which makes
  the random pitch spread compound on every call). [yes]
