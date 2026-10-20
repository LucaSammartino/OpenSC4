# parks: answered questions (archive)
<!-- Archive for [parks.md](../../parks.md). Lanes do not read this file. Open questions are in ../parks.md. -->

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**PK1:** The shipped distress threshold 25.0 is compared with a fraction. So the first monthly pass of each session
  marks every park lot run down (condition 2), and the park manager never restores it. Yet in the corpus every park
  lot is at condition 0 (one at 1). What resets the condition? [the lot developer's condition test
  (lot-developer.md §5.5) *[inferred]*. An in-game check would settle it: play one month with a park, then save,
  or watch the lot condition in the query tool]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered: there is no quirk (parks.md §4.4, PK10). The threshold is read with the percent getter (`0x005FDAC0`, × 0.01f), so the shipped 25.0 becomes 0.25 and parks are marked run down only at 25 % funding or less. The corpus's condition 0 needs no other reset.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**PK2:** The zoo escape runs only when the park list is not empty. A park budget item that names a missing
  department crashes the original. [the shipped zoo is in both groups, so the first point cannot show in the
  shipped game. *[choice]* FreeCity4 skips the charge for a missing department (parks.md §4.2)]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed). Shipped data cannot reach the crash; skipping the charge is a safe *[choice]*.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**PK3:** Park line items are never removed. A removal message releases the occupant once even when it was not
  listed. [reproduced; an empty line stays in the budget panel at 0 and 0. A budget-panel check after demolishing
  every park of one type would confirm the visible effect]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (reproduced from the exe, parks.md §4). What remains is a visual check of the budget panel, not a behaviour question.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**PK4:** park tuning with no exemplar. [threshold 0.25 and an empty curve, which gives 0, so no zoo escapes; the
  tuning is read once per object]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES with one *[choice]*. Without the exemplar `ReadTuningParams` (`0x006D5990`) reads nothing and returns false; the threshold keeps the constructor's 0.25 (`0x00B0D3D8`, `0x3E800000`) and the curve stays empty (`0x007D0410`). `Init` sets `tuningRead` whatever the result (`0x006D5B1C`–`0x006D5B21`): read once per object. The original's evaluator reads an empty curve through a null array (`0x005F4289`) and crashes, so "empty gives 0" is a *[choice]*. parks §4.1, §4.4, §6.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**PK5:** park insert when the item names a missing department, or a line item cannot be created. [steps 5–7 are
  skipped; the line item is skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes in both cases: a missing department is dereferenced at `0x006D571F` (after step 5 has already changed the item's line), and a line item that cannot be created is dereferenced by `SetDisplayFlag` at `0x006D573E`. The bracket is accepted as a *[choice]*; FreeCity4 still runs step 8 (release the building) and the zoo step. parks §4.2.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**PK6:** park monthly pass with no held department or unreadable funding. [stops after step 1]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: no null test on the department (`0x006D58C4`) or on the funding pointer (`0x006D58CA`). Stopping after step 1 is a *[choice]*. parks §4.4.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**PK7:** park zoo removal of an occupant that was not listed. [released once anyway: the literal reading]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The list removal (`0x006D5200`) frees the matching nodes without releasing; the single release follows unconditionally (`0x006D52C3` parks, `0x006D5366` zoos). parks §4.3.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**PK8:** park `Read`. [atomic; the lists are replaced without releasing their old entries]~~ **ANSWERED 2026-10-15 (re-answers10):** the lists are replaced without releasing their old entries (`0x005DB3C0` clears first): MATCHES. The original is **not atomic** (both lists read with no error test in between; the result is the stream's error state, `0x006D51EE`); atomicity is a *[choice]*. A wrong version changes nothing. parks §7.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**PK9:** the park generator. [`cRZRandomSuper` seeded with derive(S, 0xAA3FE6BC)]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES as a *[choice]*. The original builds `cRZRandomSuper(−1)` in the constructor (`0x006D5850`–`0x006D5855`, `0x00913B74`): wall clock, once per object, not reseeded by `Init`. FreeCity4's `derive(S, 0xAA3FE6BC)` follows foundations §6.3; bit 0 of the seed is ignored (RN1). parks §5, foundations §6.2.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**PK10:** in game the original park manager's threshold reads 0.25 (the code default), not the exemplar value, so
  the PK1 quirk is not observed. Why does the original not take the exemplar tuning? [the glue follows the spec
  (exemplar tuning at the first `Init`); the shadow follows the original "as without exemplar" and says so on every
  line]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS**: the spec was wrong, and the original does take the exemplar (parks.md §4.4, §6, tests 2 and 6 corrected). The threshold is read with the **percent** getter `0x005FDAC0` (`0x006D59EB`), which multiplies by `0.01f`: 25.0 becomes 0.25 exactly, which is what the game shows. The glue and the shadow should read `0xCA480830` × 0.01f; with the shipped data both give 0.25, so only a modded exemplar would differ.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**PK11:** `fc4_park_destroy` and `fc4_civic_destroy` do not release the list entries of a never-initialised object
  (as the original's destructor, CB7), which leaks the references a `Read` took if `Init` never runs. [as the spec
  says]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (parks.md, civic-buildings.md §8). The park manager's destructor (`0x006D5860`) frees the nodes of both lists (`0x00413FE0`) without releasing their entries, and the civic one does the same (CB7). A never-initialised object that read references therefore leaks them in the original too.
