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

<!-- from: core-app5b (2026-10-23) -->
- **AP16 - sections 3.1 step 8, 14.2 resource factories:** does Init inspect SetupResources' final count>=3 result, or continue after a false result? [Literal section 3.1 call ordering: invoke SetupResources and continue, since no failure branch is specified there. The direct setup_resources API returns the exact unsigned count>=3 boolean. No failure dialog, rollback, or early Init return is invented.]

<!-- from: glue-app-shadow13 (2026-10-24) -->
- **AP17 - sections 1, 6-8, 13, 15, nonmutating application shadow observation:** what supported observation contract exposes native app frame/tick counters (if any), pause/speed inputs and outputs, and the elapsed values of the five section 7 timers immediately before native OnIdle, together with its actual dispatch effects? Timer offsets alone do not specify their representation or elapsed-time accessor. How can the observer capture pre-ReadPreferences/pre-ReadCommandLine state and the consumed INI/command-line inputs without invoking these mutating startup methods again? This asks for the capture boundary, not the AP6 defaults or AP8 semantics already open. [Keep app_shadow default-off and bounded to section 9 paths, section 2 filenames and section 6.2 message-derived mode; compare at a first-city quiet point and monthly. Do not infer timer layouts, hook signatures, app counters from observer tick counts, simulator speed from app pause, or startup inputs from final outputs. No whole-owner verification claim.]

<!-- from: glue-app-shadow13 (2026-10-24) -->
- **AP18 - section 9, lazy slots 65/66:** run app-shadow13-2032-r2a (run 631, clean to 2032-01-05) finds exactly these two getters different from the literal core model on all 73 load/month observations. Native slot 65 is `C:\Users\tlori\Documents\SimCity 4\Exception Reports\`; native slot 66 is `C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition\Scripts\TestScripts\`. What are their precise base/suffix and cache initialization contracts, and how can an observer capture the independent pre-cache inputs when these caches were populated before the first city observation? Section 9 currently gives the ordinary slot58 base for Exception Reports and slot53(-1) plus `Scripts\` for slot66; AP7 only resolves separator/cache-invalidation rules. [Keep the literal core unchanged. Exclude slots65/66 from supported app/app-startup labels; expose their unresolved comparison only via additional default-off app_shadow_path_cache=1 as app-path-cache. Never seed expected paths from native cache/getter outputs or claim whole-owner verification.]
  Normal-speed confirmation: app-shadow13-2032-r2normal (run 632), clean to 2032-01-01, supported app 72/72 and app-startup 1/1; separate app-path-cache 0/73. Captured slot58 base was `C:\Users\tlori\FreeCity4\build-runs\sc4slots\slot2\`, so the literal expected slot65 ended `slot2\Exception Reports\`, while native still used Documents. Captured slot53(-1) base was the install directory; expected slot66 ended `Scripts\`, native `Scripts\TestScripts\`. These differences persist without turbo; no inferred core fix applied.

<!-- from: records-cities29 (2026-10-26) -->
- **AP19 - sections 6.2 and 15, lifecycle input observation during modal delivery:** what supported, nonmutating boundary exposes the mode-changing message before an earlier notification target enters a modal loop? In `C:/Users/tlori/FreeCity4/build-runs/suite/records-cities29-1a/maxisland-bevopolis.log`, native slot 25 returns mode 2 at 11:47:34.287 (startup), 11:47:48.980 (May), and 11:48:17.216 (June), while our message-derived model is still 0 with zero mode notifications. Our ordinary server-2 subscription receives PreCityShutdown at 11:48:49.091, then PostCityInitComplete at 11:48:49.375, immediately after modal EndModal fallbacks and just before LoadCity returns. This is delayed delivery to our target, not evidence that the app's section 6.2 transition is wrong. The public AddNotification API has no priority argument. Deferring comparisons until our late notification can discard an entire running city, and seeding mode from slot 25 would compare an output with itself; neither is applied. Blocks app/app-startup in Bevopolis, Norte Delano, South Pennoli and South Weburg from census3; Norte Delano also reproduces in records-cities29-1a. Please specify an independent early input capture or a guaranteed observation boundary that retains these samples.

<!-- from: automata-actions30 (2026-10-26) -->
- **AP20 - section 7, native wait/wakeup boundary during the s1 action acceptance:** identify the native
  path containing saved return addresses `0x00923A61` and `0x0090CF71`, its wait/wakeup contract and any
  required runtime control capture. `C:/Users/tlori/FreeCity4/build-runs/game-results/aa30-tv4-house-s1-e.log`
  lines 12432-12448 and 12515-12531 show the main thread at `win32u.dll+0x11EC` in two watchdog samples,
  with zero main-thread CPU progress, outside FreeCity4, and no OnTick for 5484 then 10531 ms.
  The unchanged house replace list and `gen27-konradshohe-s1-grow.script` completed 27/216 actions before
  the final stalled verdict on 2026-02-22; exceptions, RTC failures and reference warnings are all zero.
  DLL SHA256 prefix `2a82f565`; matching request/verdict are in main's game-queue/done and game-results.
  Is this an intended modal/message wait, and which independently observable input releases it?
  Ownership is unassigned: the current capture does not establish an automata or terrain-view call.
  [runtime observation required; stop this item without changing the application loop or watchdog]
