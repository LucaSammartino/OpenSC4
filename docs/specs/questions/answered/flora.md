# flora: answered questions (archive)
<!-- Archive for [flora.md](../../flora.md). Lanes do not read this file. Open questions are in ../flora.md. -->

## From the flora glue and census shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/flora.md` and its `docs/abi` rows. Ini: `enable = flora` (ten `flora_*` bisection switches);
shadow `flora_shadow = 1` (registry once per city, then the deterministic part of each monthly pass; the random
part of §8 is not compared). The glue's current answer is in brackets.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ1:** exemplars are enumerated with `GetAvailableResourceList` and an exemplar-type filter (the demand glue's
  pattern), not slot 18 `GetAvailableResourceListForType`, and keys are read by index (`Size`/`GetKey`), not
  `EnumKeys`. Equivalent? [assumed]~~ **ANSWERED 2026-10-14** from the exe (slot 18 `0x00973280`, its filter
  `0x00973194`, slot 17 `0x009739CB`, `EnumKeys` `0x009749CB`, `GetKey` `0x00974741`, the callback `0x006B03B0`)
  and written into flora.md §4.1 step 2 and §12. **Equivalent**, provided the filter accepts exactly the keys
  whose type is `0x6534284A` (any group, any instance): slot 18 is slot 17 with such a filter, and `EnumKeys`
  walks the same sequence that `Size`/`GetKey` index, first to last.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ2 — §4.1 step 2:** the app shutdown callback is not registered: its timing against our DLL's unload is not
  documented, and the records live for the whole process. [not registered]~~ **ANSWERED 2026-10-14** from the
  exe (the callback `0x006B1070`; the app's list and its walk in `cSC4App::Shutdown` `0x0044C130`,
  `0x0044A633`–`0x0044A650`; the framework's shutdown order `0x0087AB07`) and written into flora.md §3.2. **No
  observable original behaviour; the implementer's choice stands.** The callback only drops the map's reference
  to each record and empties the map and the type array at process exit. For the record: callbacks run inside
  the app's `Shutdown`, in registration order, after PreAppShutdown and before PostAppShutdown reach the framework
  hooks, so a DLL's callback would still be loaded when it runs.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ3 — §10.3:** `cIGZPersistDBSegment`'s record read/write is not in `docs/abi`, so saving and restoring an
  obliterated city's trees is off (`SaveObliterated` writes nothing and returns true; a restore is logged when the
  segment at +0x204 is non-null). Layout and slots? [off]~~ **ANSWERED 2026-10-14** from the exe (save
  `0x006B2760`, restore `0x006B0660`, the record-stream helpers `0x005D9DB0` / `0x005D9C90`, segment vtable
  `0x00AA2654`, stream vtables `0x00AA2280` / `0x00AA2310`) and written into flora.md §10.3, §12 and test 30.
  **The bracket (off) loses trees across obliteration.** Query the segment for `cISC4DBSegment` `0x499899DA`;
  slots 8/9 open/close a write stream, 5/6 a read stream; stream slots 7, 9, 18 = u16, u32, GetError.
  *Correction to §10.3:* each record starts with a u16 version 0 (count record: `u16 0, u32 count`; occupant
  record: `u16 0` then the occupant's own version-3 record); the reader ignores the u16.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ4:** `cISC4OccupantFilter` is not in `docs/abi`. Instead of the filter overload (slot 36) the glue calls the
  typed box query (slot 37) for flora, building, landfill and prop, then for networks keeping those with
  `GetNetworkFlag() & 0x1F4F` non-zero. Same set? [as stated]~~ **ANSWERED 2026-10-14** from the exe (filter
  vtable `0x00AB1374`, slot 3 `0x006AFD10`; box query `0x004A0670`; typed query `0x0049DD40`) and written into
  flora.md §6.4.1. **Same set; only the order differs, and nothing observes it.** Slot 37 is slot 36 with the
  manager's one-type filter; slot 36 calls only the filter's slot 3 `bool IsOccupantIncluded(cISC4Occupant*)`
  (pop 4, AL). A network occupant that does not answer `cISC4NetworkOccupant` does not block.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ5:** weather `GetMoistureType` is read as a `uint8_t` in AL (`FC4Interfaces.h`). Right? [AL]~~
  **ANSWERED 2026-10-14** from the exe (`GetMoistureType` `0x007256D0`, `GetMoistureValue` `0x00726150`, the flora
  reader `0x006B0939`) and written into flora.md §6.2. **Right for reading the original weather**: it returns
  `value >> 4` from a zero-extended byte, so EAX is 0..15 with the upper bits zero and AL loses nothing. The
  declaration should still be a 32-bit return: original flora uses all of EAX as the table offset, so a
  FreeCity4 weather must return a clean EAX (already in §13.2).

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ6:** "occupant in a visited block" (shadow) uses a closed box, edges included, like the host's box query.
  [closed]~~ **ANSWERED 2026-10-14** from the exe (box query `0x004A0670`, its cell mapping `0x0049E0E0` and clamp
  `0x0049DF00`, listing in `InsertOccupant` `0x0049EF60` / `MoveOccupant` `0x0049F3C0`) and written into flora.md
  §6.4.2 and test 37. **No, on two counts.** The host's test is strict (open) on x, y and z, so touching boxes
  are not returned. And it only looks at occupants listed in the query's 64 m index cells; an occupant is
  listed from its box shrunk by 0.1 m per side (boxes wider than 1 m), with coordinates up to 0.064 m below a
  64 m line counted in the cell above. A box whose low edge is 0.1–0.164 m below such a line overlaps the block
  below it but is not returned for it (this is FQ15).

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ7:** the shadow takes the elevation scale from the tuning exemplar's `0xC9DBDD02`; the record does not save
  it. [as stated]~~ **ANSWERED 2026-10-14** from the exe (constructor `0x006B1941`, tuning read `0x006B01B0`,
  `Write` `0x006AFEE0` / `Read` `0x006AFFE0`) and written into flora.md §7. **As stated.** Default `0.08f`;
  `16.0f / v` only when the tuning exemplar has `0xC9DBDD02`; re-read at every `Init` and on an exemplar change of
  `0x1A2FDB6A`; never saved.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ8:** blocks are never freed and the registry is never cleared at app shutdown. [as stated]~~ **ANSWERED
  2026-10-14** from the exe (shutdown callback `0x006B1070`, reload in `DoMessage` `0x006B352B`–`0x006B354B`) and
  written into flora.md §13.2. **No observable original behaviour; the implementer's choice stands.** The
  original releases the registry's references and empties the map and type array only in its process-exit
  callback (FQ2), and its exemplar-change reload already drops old records unreleased.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ9:** `cISC4FloraTypeFilter` (slot 3) is not in `docs/abi`. Slot 13 with a non-null filter inserts nothing and
  returns 0; with a null filter it inserts every key. [as stated]~~ **ANSWERED 2026-10-14** from the exe
  (`GetFloraTypesByFilter` `0x006B23A0`; the slot-14 callers `0x0048B860`, `0x006CD010`) and written into flora.md
  §11.1. **The null-filter half matches; the non-null half does not.** The original calls the filter's slot 3
  `+0x0C` `bool (u32 floraType)` (pop 4, AL) for every map key in ascending order, key 0 included, inserts the
  accepted keys and returns how many it accepted. No exe caller passes a filter, so only plugins notice; the
  fix is a one-line call per key.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ10:** `cIGZCommandParameterSet` is not in `docs/abi`. PlaceFlora reads its parameters through slot 5
  (`operator[]`): param 0 via variant slot 24 (u32), params 1–2 via slot 36 (float). Right? [as stated]~~
  **ANSWERED 2026-10-14** from the exe (`DoMessage` `0x006B3497`–`0x006B34F6`; the set's vtable `0x00AD16C8`,
  slot 5 `0x0092A162`; variant slots `0x0091560F`, `0x0091567A`) and written into flora.md §5 and §12. **Right.**
  Slot 5 `operator[](u32)` pops 4 and never returns null (a missing index gives the set's empty variant); slot 24
  `GetValUint32()` (EAX) and slot 36 `GetValFloat32()` (ST0) wrap the converting getters with a 0 default.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ11:** the cheat notification uses id `0x095BDC9C` as the spec says, but the budget and simulator glue pass 0.
  Which is right? [`0x095BDC9C` for flora]~~ **ANSWERED 2026-10-14** from the exe (flora `0x006B2C77` and
  `0x006AFDDD`; the cheat manager's `AddNotification2` `0x009212A6` and dispatch `0x00920F34`–`0x00920F3E`; budget
  `0x00693135`; simulator `0x007033B1`) and written into flora.md §4.1 step 11. **The cheat ID is right, for all
  three.** The original budget and simulator also pass their own cheat IDs. ID 0 means "every cheat": the
  manager delivers a cheat to each pair whose ID matches it or is 0, so the budget and simulator glue with 0
  receive all cheats (and must remove the pair with the same ID they added).

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ12:** `RegisterCommand`'s parameter info is null, as the spec says. [null]~~ **ANSWERED 2026-10-14** from
  the exe (flora `0x006B2A0C`–`0x006B2A1F`; command server slot 6 `0x0092B075`, slot 8 `0x0092AD10`) and written
  into flora.md §4.1 step 3 and §12. **Null is right.** The name is a plain `const char*` (`"PlaceFlora"`), the
  call is made only when the message server exists, and its result is ignored. With null info the server writes
  a default help text (`"Format: PlaceFlora\nOutput: <none>"`) when its help flag is set.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ13:** `ToCoreAll` leaves out parent-cohort properties (existing plugin behaviour). Does flora need them?
  [left out]~~ **ANSWERED 2026-10-14** from the exe (`AddStaticFloraData` `0x006B12D0`; the `0x10` test
  `0x005FD480` in `0x006B03B0`) and a scan of the shipped `.dat` files, written into flora.md §3.3. **The original
  includes them.** Every flora property, and the `0x10 = 0x0F` test itself, is read through the effective list
  (data-formats §4.4). The 22 shipped flora exemplars have no parent cohort, so the shipped game is unaffected;
  a plugin whose flora values sit in a cohort would get defaults (or not be registered) if they are left out.
  *[choice]* include them.

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ14 — §6.11:** adding a record whose next type is 0 set key 0's seed type to it in the test harness. Does
  §6.11 apply when next is 0? [the test avoids the case]~~ **ANSWERED 2026-10-14** from the exe
  (`AddStaticFloraData` slot 6 `0x006B3570`, loop `0x006B3725`–`0x006B3776`; `Init` load `0x006B299D` before the
  key-0 insert `0x006B2BAA`; reload `0x006B352B`–`0x006B354B`) and written into flora.md §6.11 and §3.2. **Yes, it
  applies; there is no special case for 0.** The harness result is the original's. In the game it cannot happen:
  key 0 is never in the map during a load (the first load precedes its insert; the reload clears it and does not
  put it back), and nothing reads key 0's seed type. The test may keep avoiding the case or expect the capture.
  Side correction in §3.2: after a reload, `IsFloraTypeValid(0)` is false until the next `Init`.
- Not done (each logged once): obliterated-city save/restore (FQ3), the app shutdown callback (FQ2),
  `GetFloraTypesByFilter` with a non-null filter (FQ9).

<!-- from: From the flora glue and census shadow (2026-10-14) -->
- ~~**FQ15 — first in-game shadow (run 80, coordinator):** the registry is identical and about 2,600 trees match each
  month, but exactly one tree differs in 23 of 94 months: 21 times its birth date (ours predicts a rebirth, e.g.
  birth 2461102 kept by the original where ours predicts 2461224) and twice its state (state 0, ours 2). What in
  §6.7.2 keeps a tree from being reborn (a visit count, a random draw, the block's iterator position)?~~
  **ANSWERED 2026-10-14** from the exe (occupant manager slot 15 `0x0049E0E0`, listing `0x0049EF60` /
  `0x0049F3C0`, query `0x004A0670`; see FQ6) and the run-80 differences, written into flora.md §6.7.1 (table of the
  six trees, example) and §16 test 38. **Nothing in §6.7.2: the tree is not visited.** The monthly block query
  goes through the occupant manager's 64 m index (§6.4.2). A box wider than 1 m whose low edge is 0.1–0.164 m
  below a 64 m line is listed only from the cell above, so the block just below the line overlaps it but does
  not return it. Six trees in run 80 sit in that window (one per month, 23 tree-months in all; e.g. Oak Tree
  clump 5 `0x14C279F4`, box z from 319.844, is visited by blocks (53, 10) and (54, 10) only). Ours also visited
  them from the block below, and since wild clumps live 2–42 days, every such extra visit was a rebirth.

## From applying the aura, education/health and flora answers (filed by the coordinator from the `apply5` lane's report, 2026-10-14)

AU1–AU15, EH1–EH14 and FQ1–FQ15 are applied (the aura shadow follows the original's state field; the flora obliteration
record goes through record streams, switch `flora_obliteration`). Not applied: a call-order test for EH1, and the aura
shadow's riot roll (wall-clock RNG). The current answer is in brackets.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ4:** FQ3 save: is flag 1 toggled only when the obliteration record opens? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4FloraSimulator::SaveObliterated` `0x006B2760`: open test `0x006B2886`,
  clear, write and set `0x006B2890`–`0x006B28AB`, release `0x006B28B2`) and written into flora §10.3. Yes: the flag is
  cleared and set again only in the branch taken when the record opened. The occupant's list reference is released
  either way. The bracket matches.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ5:** FQ3 restore: is the record closed right after `Read`, before `Init` and `InsertOccupant`? [yes]~~
  **ANSWERED 2026-10-14** from the exe (`cSC4FloraSimulator::LoadObliterated` `0x006B0660`: the record helper is built
  at `0x006B06DA` and destroyed at `0x006B073D`) and written into flora §10.3. No: the helper lives for the whole
  iteration, so the record is closed only after `Read`, `Init`, `InsertOccupant` and `Release`. Neither `Init` nor
  `InsertOccupant` touches the segment, so the order cannot be seen. The bracket (yes) differs from the original; either
  order is safe.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ6:** FQ3: a failed `GetUint32` of the count. [the count stays 0]~~
  **ANSWERED 2026-10-14** from the exe (`LoadObliterated` `0x006B0660`, count := 0 at `0x006B067E`;
  `cSC4DBSegmentIStream::GetUint32` `0x005D1A20`; record reads `0x00975DE5`, `0x0096D967`, `0x0096DAF8`) and written
  into flora §10.3. Yes: a failed stream skips the read, and the record checks its size before it copies anything, so
  the count keeps the 0 it starts with and nothing is restored. The bracket matches.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ7:** FQ6: is the 64 m listing clamped to 0..191? [not clamped; only the query is]~~
  **ANSWERED 2026-10-14** from the exe (`InsertOccupant` `0x0049EF60`, listing loop `0x0049F0B6`–`0x0049F1BA`;
  `MoveOccupant` `0x0049F3C0` through `0x0049DAC0`; the query's clamp `0x0049DF00`) and written into flora §6.4.2. Not
  clamped: the listing keeps each cell's low byte and never tests it against 192, so an edge more than a city width
  outside the city writes past the index or is listed nowhere. The bracket matches; for such boxes clamping is a
  *[choice]*, recommended.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ8:** A bound on the obliteration open attempts. [choice: at most `MAX_OBLITERATED` = 2^24; the original has no
  bound]~~
  **ANSWERED 2026-10-14**: no original behaviour (the original makes `count` open attempts with no bound,
  `0x006B06C6`–`0x006B0749`); the implementer's choice stands. Noted in flora §10.3.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ9:** Flora shadow: which box gives the 64 m listing? [choice: the census box taken before the pass]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. Noted in flora §6.4.2 with the
  original's rule: the listing is made from the box at `InsertOccupant` or `MoveOccupant` time and kept in the occupant
  (`0x0049F094`) until the next move.

<!-- from: From applying the aura, education/health and flora answers (2026-10-14) -->
- ~~**AQ10:** Plugin: `OpenOStream`/`OpenIStream` returns false but writes a stream pointer. [treated as not open;
  nothing released]~~
  **ANSWERED 2026-10-14** from the exe (record helpers `0x005D9C90` / `0x005D9DB0`, open test `0x005D9670`, destructors
  `0x005D9C30` / `0x005D9D50`) and written into flora §10.3. Not open: the helper keeps the segment only on success, and
  the open test needs both. But the written pointer stays in the helper's stream reference, and the helper releases it
  once when it is destroyed, without a close call. The bracket's "nothing released" differs; *[choice]* release it once,
  as the original does (recommended).

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**AQ12:** AQ10: how should the stray reference from a failed `OpenOStream`/`OpenIStream` be audited? [as
  acquired, then released as created-for-us]~~ **ANSWERED 2026-10-15 (re-answers10):** glue, with the original's ownership as the guide: an `Open*Stream` out-pointer is a reference the callee hands to the caller (the game's segment gets it by `QueryInterface` and drops its own, `0x005D27B3`–`0x005D27C0`). The helper adds none and releases it once. Audit it as handed over (created for us) at the call, whatever the call returned, and the helper's release as that reference's release. The bracket's "acquired" is right only if the audit counts both kinds together. flora §10.3.

<!-- from: From applying the REH, flora, residential and flammability answers (2026-10-14) -->
- ~~**AQ13:** AQ7: is each end of a listing clamped to 0..191 on its own? [yes]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES the query's rule, which is the model for the recommended clamp: `0x0049DF00` clamps each of the four values on its own, as a signed 32-bit cell number, to ≥ 0 and then ≤ 191. The original's listing itself does not clamp (AQ7). flora §6.4.2.
