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

<!-- from: core-application4 (2026-10-22) -->
- **AP6 - sections 3-5, 12:** what are the unpublished startup option sources/defaults, version construction, user-data selection, cursor/OS setting and plugin-date checks, language registration, log setup, watcher path/creation rules, Lua option/switch precedence and FindTestScript success/failure contract, service allocation/registration failure behavior, and exact helper destruction order/free discipline? [Portable runner stops at named missing-owner boundaries; external primitive failures stop without guessed rollback; registrations retain separately and local created references are released; caller supplies watcher tokens/paths. Lua uses configured option before switch, erases a present switch once selected; helper destruction remains an explicit blocking boundary. No complete owner claim.]
<!-- from: core-application4 (2026-10-22) -->
- **AP7 - section 9:** what exact path-append separator rules apply to empty bases and bases already ending in slash/backslash, and does a language subfolder have a trailing separator? [Literal Windows folder append: insert backslash only when needed, append trailing backslash; preserve existing separator bytes. Lazy caches are never invalidated by UserDir changes. AP3 aliases remain offset-named.]
<!-- from: core-application4 (2026-10-22) -->
- **AP8 - section 8:** publish resolution acceptance checks, remaining UserDir validation, cross-option application order, numeric overflow/invalid-input rules, float-prefix rules and single-character switch colon handling. [Use table order, existing cRZCmdLine character/named lookup, unsigned 32-bit wrapping numeric prefixes, English float prefix with no mutation on failed conversion. Resolution/UserDir final acceptance remain explicit unresolved results with no preferences/path commit; l alone strips a leading colon as published.]
<!-- from: core-application4 (2026-10-22) -->
- **AP9 - section 6.1:** does cheat text contain just arguments or the cheat name too, are GP synonyms accepted, and what does GP's "apply it" call do with current focus? [Parse text as arguments using existing portable delimiter parser, accept exact lower-cased on/off only; update both pause-enable bytes, then return PreferencesChanged boundary instead of inventing an active-state query.]
