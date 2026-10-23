# foundations: answered questions (archive)
<!-- Archive for [foundations.md](../../foundations.md). Lanes do not read this file. Open questions are in ../foundations.md. -->

<!-- from: verify-variants4 return-channel gate and first native run (2026-10-22) -->
- **HG6 - section 17.1 slots 5/6/7 return channel and clone identity:** The ABI gate describes Erase,
  CopyFrom and CreateCopy as eax-or-void. What return values, if any, are contractual? Does SDK CreateCopy
  return a distinct COM implementation of the same published 127-slot cIGZVariant interface? The owned
  result in vv4-original-a567f1-1820 had a different native vtable from the published borrowed source,
  was released to zero with ref_warnings=0, and was refused before testing. [Discard nominal bool results;
  judge CreateCopy by its independent owned non-null pointer and supported-image interface validation,
  CopyFrom/Erase by resulting state. No unpublished clone address/layout is called or baked into code.]
  **ANSWERED 2026-10-23 (re-variant-storage4):** DIFFERS — normal native Erase/CopyFrom/CreateCopy
  explicitly set AL=true; upper EAX is not contractual. CreateCopy constructs a distinct standalone
  variant whose first 127 table entries differ from the embedded source only in identity/ref slots
  0/1/2, and publishes one owned reference after AddRef. Allocation failure is not a qualified clean
  false/null result. Rules and original addresses are in [variant-storage §§2/5](../../variant-storage.md#5-createcopy-and-return-channels),
  linked from foundations §§9.1/17.1. No native rerun or unpublished verifier address was added.

## From the protection fix: saved grid refs and their geometry (filed by the coordinator from the lane's report, 2026-10-14; merged)

Run 70 found the fire replacement's grid at 0×0 after a load. The fire record's grid ref points to a grid whose own
record is read later, so the grid is still empty during the simulator's `Read`. The host grid now keeps a geometry
only once it is non-empty. The current answer is in brackets.

<!-- from: From the protection fix: saved grid refs and their geometry (2026-10-14) -->
- ~~**HG1 — foundations (load order):** are all grid records read before any simulator's `Init`, so that the first
  non-empty geometry is the final one? Which order does the class store read records and resolve refs in, relative
  to `Read` and `Init`? [assumed yes]~~ **ANSWERED 2026-10-14** from the exe (`LoadCity` runs the store load at
  `0x0045574C` → `0x005D2080` → `0x005D38D0`, classes from a sorted map, `Read` at `0x005D3C63`; `cSC4City::Init`
  only at `0x004558A2`, after PreCityInit at `0x00455898`) and written into foundations §3.7 ("When objects, grids
  included, are read"). **The bracket matches: yes.** Every saved object is read in the store load, in ascending
  class ID and record order, before the city's `Init` runs. A reference only creates an empty object; its `Read`
  comes when the walk reaches its entry. Subsystems with class IDs below `0x49B9E602` (fire protection among them)
  see their grids empty during `Read`; all see them final by `Init`. Caveat: a grid saved as 0 × 0 stays empty.

## From the save-reference audit (filed by the coordinator from the `lvrefs` lane's report, 2026-10-14)

Run 88 (save, then reload, fourteen replacements): after an in-session save, land value's release of its three grids at
Shutdown was the last reference; on a loaded city without a save, the game still holds a reference at Shutdown. The
lane now treats every reference a glue holds from `Read` as its own (`FC4_RELEASE_CREATED`), which is correct either
way.

<!-- from: From the save-reference audit (2026-10-14) -->
- ~~**SV1 — foundations §2.7:** which references does the game's load (the persistent resource manager's object table,
  `ReadGZSerializable`) keep on the objects it creates or resolves, and when does it drop them: at the end of the
  load, at the next save, or at city Shutdown? [our reading: the load keeps one until the next save; marked in the
  code as a reading, not a finding]~~ **ANSWERED 2026-10-14** from the exe (serializer resolver 0x005D2DD0, reset
  0x005D3250, segment Close 0x005D20C0, SaveCity 0x00455E20 at 0x0045641B/0x00456449/0x0045662F, PartialCloseCity
  0x00454A60 at 0x00454AF3) and written into foundations §2.7 "References the load keeps".
  - The table is the object table of the city segment's COM serializer. It keeps exactly one reference on each object
    the load **creates**, none on an object it only resolves; the loader's own local is released after `Read`, and the
    typed helper nets +1 for the destination.
  - It drops them all when the segment that loaded the city is closed: at the **next save** (the save switches to a
    fresh read-only segment with an empty table), or, if no save came first, at **city close after the city's
    `Shutdown`**, or on a failed load. Not at the end of the load.
  - The bracket matches, with one addition: in a loaded city that is never saved, the table still holds its reference
    during every `Shutdown`, so a subsystem's release there is not the last one. Treating every reference from `Read`
    as the glue's own stays correct.

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**AO1:** Foundations §13.4 ranks 2 and 5: what AddAgent names do the two port developers and the commercial
  simulator use? [they count as unranked and keep registration order ahead of the month tail]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4PortDeveloper::Init` `0x006EA950`, string `0x00A89650`;
  `cSC4CommercialSimulator::Init` `0x0069EF90`, string `0x00A89920`) and written into foundations §13.4 (tail agent
  names). Both port developers register `"cSC4PortDeveloper"` at level 6 through their shared base `Init`; the
  commercial simulator registers `"cSC4CommercialSimulator"` at levels 0, 10, 4, 6. **The bracket differs**: with the
  names known, rank them (both port developers share rank 2; their mutual order is free, *[choice]* registration
  order) instead of leaving them unranked.

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**AO2:** LG6 with the game's own simulators: the month order is heap-address order, which the glue cannot change.
  [the order is logged at AddAgent, with a WARNING if landfill would run before pollution or the budget]~~
  **ANSWERED 2026-10-14** from foundations §13.1 (the original's set is ordered by target pointer) and written into
  foundations §13.4. No original behaviour can be imposed on the original simulator; the glue's rank follows its
  object's address. **The bracket stands** as the minimum. *[choice]*: allocate the glue's month target from the game's
  own heap at the original object's place in city `Init`, which reproduces the original's usual placement but cannot
  guarantee it.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**RN1:** foundations §6: in a mutation check, session seeds `0x1234` and `0x1235` gave identical park draws. Since
  `derive` is an XOR, the generator seems to ignore bit 0 of its seed. Does the original `cRZRandomSuper` (or its
  seeding) also drop bit 0, and does it matter for any other generator? [not investigated]~~ **ANSWERED 2026-10-15 (re-answers10):** yes, and it is the original's own behaviour: `cRZRandomSuper::Seed` (`0x00913A72`) ORs the seed with 1 (`0x00913A97`) before the seeding loop, after the clock test, so `s` and `s XOR 1` give identical streams. This matches our generator. `cRZRandom::Seed` (`0x00913667`) keeps all 32 bits. It matters only for `cRZRandomSuper` users (the file-local generators of §6.3, the park generator, …). Seed-mutation tests should flip another bit (e.g. `S XOR 2`), and `derive`'s XOR 1 fix-up is invisible to them. foundations §6.1.

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**AO3:** AO1 (foundations §13.4): the two port developers share the name `cSC4PortDeveloper` and rank 2. Is their
  relative order registration order? [yes, registration order]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES as a choice (foundations.md §13.4). The original ranks both agents by heap address and the logs do not separate them, so there is no original order to follow. Neither agent reads the other's results (separate port lists and history keys, no random draws, additive demand updates); the only visible difference is the order of two upgrade advisor messages in the same month. Registration order is confirmed.

## From the session seed (filed by the coordinator from the `seeding` lane's report, 2026-10-16; merged `77b7909`)

seeding implemented foundations §6.3 across every merged core that owns a generator (`engine/core/src/seeding.rs`,
`seeding::keys`, ABI 64) and the plugin's session seed (`session_seed = <u32>`, else the wall clock at game start). The
current choice is in brackets.

<!-- from: From the session seed (2026-10-16) -->
- ~~**RN2:** foundations §6.3 and growth-developer.md §7: a decline key whose coordinate seed is `0xFFFFFFFF` makes `Seed`
  read the clock (growth-developer.md §7), but §6.3 says coordinate-seeded temporaries are unchanged. Is that case a
  clock-seeded temporary of the owning class, and is this the intended K? [derive(S, `0x0990C138` XOR n), n counting such
  reads per developer object, not saved; `seeding::keys::growth_decline_clock`, `growth_developer/simulator.rs`]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (foundations §6.3, growth-developer.md §7). `cRZRandom::Seed` (`0x00913667`) reads the clock for a seed of `0xFFFFFFFF` whatever the caller meant, so the decline key in that case is a clock-seeded temporary of the growth developer, and `derive(S, 0x0990C138 XOR n)` with `n` counting such reads per developer object (not saved) is the §6.3 form. §6.3 now says so.

<!-- from: From the session seed (2026-10-16) -->
- ~~**RN3:** foundations §6.2 / §6.3: the budget (RiskyMoney) and pollution-effects generators are file-local statics in the
  original, seeded once per process. In our cores each simulator object owns one, so the stream restarts at every city
  load in the same process (crime and the development cores already keep theirs process-wide). Should budget and
  pollution keep their generator across city loads like the original, or is a restart at each load an acceptable
  *[choice]*? [restart at each load]~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** (foundations §6.3). The original seeds both file-local generators once per process and never again: the budget's `0x00B47B78` is referenced only by its static initialiser (`0x00A749D2`) and its draws (`0x00693544`–`0x006935C6`), the pollution effects' `0x00B49BD0` only by `0x00A74E42` and its draws (`0x006E668D`, `0x006E66DC`). The stream therefore continues across city loads in one process, as crime and the development cores already do in FreeCity4; §6.3 now states that file-local generators are process-wide. Restarting at each load is not what §6.3 specifies.
