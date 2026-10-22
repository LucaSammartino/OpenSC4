# audio-ambience: open questions
<!-- Questions about [audio-ambience.md](../audio-ambience.md). Append new entries at the END; when one is answered, move it to answered/audio-ambience.md. Rules: README.md. -->

<!-- from: re-audio4 (2026-10-21) -->
- **AA4 - section 3.1:** name occupant types `0xA823821E` and `0xC7CC75F1` of the init walk. (re-audioambience4: `0xA823821E` is the prop type; `0xC7CC75F1` is set by the
  `cSC4TrafficAutomaton`-derived constructor `0x005628F0` at `+0x28`; class name still open.)

<!-- from: core-audio-inputfix4 (2026-10-22) -->
- **AA10 - section 3.6 alarm creation:** the original does not null-check the clock in `_AddAlarmOccupant` (crash); what should a replacement do when no clock exists at creation (create the entry with `last = 0.0`, or refuse it)? [entry created, `last = 0.0`]
<!-- from: core-audio-inputfix4 (2026-10-22) -->
- **AA11 - section 3.2:** does the second move set-position (`0x004E8C70`, entered when `0xCA19D7CA` reads) test the sound system `+0x20` itself, and is it reached when the holder lacks `0x6A36AFAC`? [both posts independent, each only with the sound system present]
