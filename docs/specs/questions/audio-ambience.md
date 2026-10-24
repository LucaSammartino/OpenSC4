# audio-ambience: open questions
<!-- Questions about [audio-ambience.md](../audio-ambience.md). Append new entries at the END; when one is answered, move it to answered/audio-ambience.md. Rules: README.md. -->

<!-- from: core-audio-inputfix4 (2026-10-22) -->
- **AA10 - section 3.6 alarm creation:** the original does not null-check the clock in `_AddAlarmOccupant` (crash); what should a replacement do when no clock exists at creation (create the entry with `last = 0.0`, or refuse it)? [entry created, `last = 0.0`]
  (re-audio-final-gaps4, 2026-10-22, native fact only: `o` is AddRef'd at `0x004EA6D5`; the null test at `0x004EA6DE`
  skips only the clock AddRef; `0x004EA6E9` then reads through the null clock (access violation at address 0), no
  entry; neither caller tests the clock. The binary has no non-crashing behaviour to copy, so this stays a
  policy decision for the coordinator/user, not an RE answer.)

<!-- from: core-audio-ambience5 (2026-10-23) -->
- **AA12 - sections 5 and 8.4 BeginUpdate rectangle:** is the layer's 16-byte attention rectangle the world-space f32 rectangle or the even cell-coordinate rectangle? [literal attention rectangle: world-space WorldRect; the cell rectangle is used only for iterator seeding]

<!-- from: core-audio-ambience5 (2026-10-23) -->
- **AA13 - section 3.3 alarm removal:** when duplicate alarms exist or properties have changed since registration, does removal match occupant and kind, occupant and saved sound ID, or all entries for the occupant; is only the first match removed? [literal singular entry: remove the first entry matching occupant and kind, in list order]

<!-- from: core-audio-close11 (2026-10-24) -->
- **AA14 - layer construction, lifetime and uncovered slots:** §8 inventories slots 0–28 but
  gives no complete constructor/QI/AddRef/Release/destructor, StaticInit/StaticShutdown, Shutdown,
  DoMessage, Enable, CanHandleOccupant, Insert/Move/RemoveOccupant, loop add/remove, freshness
  load/add/remove and score-master-volume bodies. Please specify defaults (including initial loop
  ID and process cell cache), service/reference acquisition/release, Init's first-failure/retry
  state and resource/player ownership for these routines. In §8.3 does repeated CreateFilters
  clear old lists/counters as well as bit vectors? [Implement §8.2–8.7 covered algorithms as
  explicit stages; no guessed native constructor/lifetime/score-loading or successful full Init.]
