# application: open questions
<!-- Questions about [application.md](../application.md). Append new entries at the END; when one is answered, move it to answered/application.md. Rules: README.md. -->

<!-- from: re-application4 (2026-10-21) -->
- **AP3 - sections 2 and 9:** the export names for cISC4App slots +0xC8 (`GetCDAppDirectory`, which returns a
  byte), +0xE4, +0xE8 (`GetRegionsDirectory`) and +0xF0 do not match their use. Which slot returns which
  folder (user base, regions), and what is the byte at +0x668?
- **AP4 - section 1:** what interface is the +0x08 face (vtable `0x00A86C0C`, slot +0x0C `0x004559E0`), and
  what do the primary slots 4 and 5 (`0x00449420`, `0x00449470`) do?
  (partial, round 3: slot +0x0C `0x004559E0` behaves as a window-message handler for the new-city terrain
  button, section 11.15; the interface name is still unconfirmed.)
- **AP5 - sections 3, 3.1, 5:** exact behaviour still marked approximate: the `Restart` wait loop in
  PreFrameWorkInit, Init step 4 (config-rule manager request and the +0xF0C byte), the `RZ::MS` call in
  Shutdown, and the `RZ::GS` service whose slot +0x44 Quit calls twice.
