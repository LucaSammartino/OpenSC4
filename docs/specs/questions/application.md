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
- **AP5 - sections 3.1, 5.2:** exact behaviour still marked approximate: Init step 4 (config-rule manager
  request and the +0xF0C byte) and the `RZ::GS` service whose slot +0x44 Quit calls twice. (The `Restart` wait
  and the `RZ::MS` call in Shutdown are now in sections 3 and 5.1, re-app 2026-10-22.)

<!-- from: core-application4 (2026-10-22) -->
- **AP6 - sections 3-5, 12:** what are the unpublished startup option sources/defaults, constructor flags/periods/thresholds, version construction, user-data selection, cursor/OS setting and plugin-date checks, language registration, log setup, watcher path/creation rules, Lua option/switch precedence and FindTestScript success/failure contract, service allocation/registration failure behavior, and exact helper destruction order/free discipline? [Portable runner stops at named missing-owner boundaries; external primitive failures stop without guessed rollback; registrations retain separately and local created references are released; caller supplies watcher tokens/paths. Lua uses configured option before switch, erases the switch only when it supplies the selected script; missing resolution executes nothing. Neutral Rust composition state is not the native constructor. Helper destruction remains an explicit blocking boundary. No complete owner claim.]
<!-- from: core-application4 (2026-10-22) -->
- **AP7 - section 9:** what exact path-append separator rules apply to empty bases and bases already ending in slash/backslash, and does a language subfolder have a trailing separator? [Literal Windows folder append: insert backslash only when needed, append trailing backslash; preserve existing separator bytes. Lazy caches are never invalidated by UserDir changes. AP3 aliases remain offset-named.]
<!-- from: core-application4 (2026-10-22) -->
- **AP8 - section 8:** publish resolution acceptance checks, remaining UserDir validation, cross-option application order, numeric overflow/invalid-input rules, float-prefix rules and single-character switch colon handling. [Use table order, existing cRZCmdLine character/named lookup, unsigned 32-bit wrapping numeric prefixes, English float prefix with no mutation on failed conversion. Resolution/UserDir final acceptance remain explicit unresolved results with no preferences/path commit; l alone strips a leading colon as published.]
<!-- from: core-application4 (2026-10-22) -->
- **AP9 - section 6.1:** does cheat text contain just arguments or the cheat name too, are GP synonyms accepted, and what does GP's "apply it" call do with current focus? [Parse text as arguments using existing portable delimiter parser, accept exact lower-cased on/off only; update both pause-enable bytes, then return PreferencesChanged boundary instead of inventing an active-state query.]

<!-- from: re-app (2026-10-22) -->
- **AP10 - section 4.2:** the unnamed system services (`0x276C784B`, `0x94AF390C`, `0xA499932C`, `0x66B217A1`,
  `0x84174448`, `0x2AC44AE7`, `0xFAC0E21A`, constructors `0x0078D050` and `0x005C22F0`) and the GZCOM class
  `0x8A433690` and call `0x004DFA40` of PostFrameWorkInit. [as read]

<!-- from: re-app (2026-10-22) -->
- **AP11 - sections 8, 13.2:** what the preferences `+0x470` (`-sim`) and `+0x498` (`-RDD`) and the render
  properties setting `0x25` (`-AutoZoomChange`) control. [as read]

<!-- from: core-app5 (2026-10-23) -->
- **AP13 - sections 12, 14.2-14.3:** publish the Resources.ini packed-switch section and class-ID syntax, malformed-entry policy, DB directory filters/order, full resource-factory list, font/control/cursor setting tables, and GetAppIniFilePath resolution. [Implement the published enumeration/substitution summary with explicit paths/factory/packed-option inputs; preserve class bytes. Unknown details remain open, with no full resource/window setup claim.]
  Partial 2026-10-23 (re-app-ap5): packed command switches/group syntax, hexadecimal key/class syntax, all fifteen factory mappings and INI path resolution published. Malformed-entry behavior, precise directory enumeration and full font/control/cursor tables still need completion.
<!-- from: core-app5 (2026-10-23) -->
- **AP14 - sections 2, 11.10, 11.16, 13.3:** publish SC4CreateSavePathForCity's naming rule, slot 29/31 and folded getters' contracts, the twelve/fourteen video setting mappings and preference-application ordering. Does the SavedCity stub concatenate the global suffix or resolve under the application directory as section 14.3 summarizes? [Save requires an explicit independently resolved target and otherwise returns SavePathNaming before mutation. Keep literal section 11.10 concatenation. Video bodies and undescribed interface slots remain open under section 12.]
  Partial 2026-10-23 (re-app-ap5): getter contracts, video mapping/order/widths, filename collision/reuse rule and literal SavedCity concatenation published. Full DoPreferencesChanged ordering and platform filename-sanitizer details remain to finish.
