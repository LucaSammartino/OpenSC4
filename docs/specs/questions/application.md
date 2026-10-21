# application: open questions
<!-- Questions about [application.md](../application.md). Append new entries at the END; when one is answered, move it to answered/application.md. Rules: README.md. -->

<!-- from: re-application4 (2026-10-21) -->
- **AP1 - section 8:** which field or preference does each `ReadCommandLine` option (`0x0044EED0`) write,
  and with which value? Section 8 lists only the option names and value words; the per-option effects
  (for example `f`/`w` full-screen versus windowed, `d` driver, `CPUCount`, `UserDir`) need a full read of the
  3986-byte body.
- **AP2 - section 7.1:** in `OnIdle` (`0x00449C70`), what does the 300 ms gate (+0x390 against `0x12C`) do on
  the path that sets the idle value to 60, and what does the tail that tests the paused byte +0x5D1 do? Are
  the idle message data and the set-back exactly as section 7.1 states?
- **AP3 - sections 2 and 9:** the export names for cISC4App slots +0xC8 (`GetCDAppDirectory`, which returns a
  byte), +0xE4, +0xE8 (`GetRegionsDirectory`) and +0xF0 do not match their use. Which slot returns which
  folder (user base, regions), and what is the byte at +0x668?
- **AP4 - section 1:** what interface is the +0x08 face (vtable `0x00A86C0C`, slot +0x0C `0x004559E0`), and
  what do the primary slots 4 and 5 (`0x00449420`, `0x00449470`) do?
- **AP5 - sections 3, 3.1, 5:** exact behaviour still marked approximate: the `Restart` wait loop in
  PreFrameWorkInit, Init step 4 (config-rule manager request and the +0xF0C byte), the `RZ::MS` call in
  Shutdown, and the `RZ::GS` service whose slot +0x44 Quit calls twice.
