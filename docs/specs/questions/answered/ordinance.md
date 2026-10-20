# ordinance: answered questions (archive)
<!-- Archive for [ordinance.md](../../ordinance.md). Lanes do not read this file. Open questions are in ../ordinance.md. -->

<!-- from: From the save-reference audit (2026-10-14) -->
- ~~**SV2 — ordinance §9.1:** does anything other than the ordinance simulator hold a foreign (plugin) ordinance read
  from a save? [assumed no; released as ours]~~ **ANSWERED 2026-10-14** from the exe (the only `cISC4Ordinance` QIs
  are the ordinance's own 0x0080C924 and the simulator's `Read` 0x0080E96B; the `GetOrdinanceByID`/`ByName` call sites
  listed in §9.1) and written into ordinance §9.1 "Who else holds a loaded ordinance".
  - Yes, one other holder: the city segment's serializer object table (foundations §2.7, SV1) keeps one reference on
    each ordinance the load created, until the next save or, in a city never saved, until city close after `Shutdown`
    and after the city is released.
  - No other exe code holds one: the messages carry the simulator, and every `GetOrdinanceByID`/`ByName` caller (LUA,
    residential, budget window) uses the pointer at once without AddRef or storing it.
  - The bracket "assumed no" differs, but "released as ours" is correct: the simulator's reference is its own. In an
    unsaved loaded city the table's release destroys the ordinance after the simulator, which nothing can observe.
