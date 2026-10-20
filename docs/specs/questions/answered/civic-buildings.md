# civic-buildings: answered questions (archive)
<!-- Archive for [civic-buildings.md](../../civic-buildings.md). Lanes do not read this file. Open questions are in ../civic-buildings.md. -->

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CB1:** Lot placement and exclusion IDs. The civic simulator has no preview-occupant test, so a preview occupant
  of a building with `0xEA2E078B` adds that ID and then erases it. If the preview's removal came after the real
  building's insertion, the ID would be lost and the building could be placed twice. [in the shipped game a placed
  reward building is disabled in the menu, so either the preview's removal comes first or the preview is never
  inserted. FreeCity4 must keep the original message order. An in-game check (place the Mayor's House, look at the
  menu) with a message log would settle it]~~ **ANSWERED 2026-10-16 (re-answers25):** Settled statically: the preview is never inserted, so the ID cannot be lost (the bracket's second alternative MATCHES). The place-lot tool's preview building is made by `cSC4ViewInputControlPlaceLot::GetBuildingPreviewModel` (`0x004C0680`; preview property `0x029244DB` set at `0x004C085C` and `0x004C0D6D`) and only drawn; the tool's `InsertOccupant` calls (occupant manager `(*0x00B43D0C)+0x50`, in `PlaceLot` around `0x004C1740`) insert the real lot's occupants, which carry no preview property. civic-buildings §4.3. In-game check if wanted: breakpoints on the exclusion insert (`0x0069E2DD`, call `0x0045D650`) and erase (`0x0069E2F9`, call `0x0045D460`) while hovering with the Mayor's House tool: expect no hit until the click places the lot, then one insert.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CB2:** A lot that returns to state 3 (for example after abandonment) is charged a second time and listed in
  `maintained` a second time. Does the shipped game send state 3 twice for a civic lot? [reproduced as is. The
  corpus holds 7 `maintained` entries in 5 cities and no duplicate. A lot-state message log for an abandoned and
  restored civic building would settle it]~~ **ANSWERED 2026-10-16 (re-answers25):** Reproduce as is (MATCHES); a second state 3 is not reachable for a civic (purpose-0) lot in the shipped game. Lot slot 34 `SetState` (`0x006C3100`) posts `0xE8BDA2E7` only on a change. Its callers: AbandonBuilding (4) and ReoccupyBuilding (3) and ChangeBuilding (3) in the tract developers, which act only on lots whose profile purpose is their own (no developer has purpose 0); ClearLot (1), EndDemolishLot (6), StartConstruction (3) and the construction occupant (3), which start a new building. The lot developer only reads a purpose-0 lot's condition. civic-buildings §4.4. In-game check: log `0xE8BDA2E7` with data2 = 3 per lot (or break at `0x0069E343`, the civic activation dispatch) while a civic building loses and regains power, water and funding: expect one message per lot.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CB3:** Status bytes 4, 5 and 6 occur in the corpus (15, 8 and 32 entries). The SDK calls bit 4 "advisor event
  data set". Who sets it, and does anything read it back? [carried through `Read` and `Write` unchanged; the
  advisors write through `GetConditionalBuildingStatus`. A trace of the writers of the status byte would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered (civic-buildings.md §2). Bit 4 is the reward advice's "already announced" mark: advisors.md §6.6 sets status 6 when the condition first returns 1 and fires only while bit 4 is clear. Later results keep it (a string gives 4, number 0 gives 5), which accounts for the corpus values 4, 5 and 6. The advisor reads it back; nothing else does.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CB4:** `buildingCount` is a u16 that wraps. A removal without a matching insert gives 65,535: for example a
  building inserted before its status entry existed, or removed before activation. [reproduce the wrap. Counts
  above 0 occur in only 13 entries of 5 cities, and none has wrapped. A save that loads with a building but
  without its status entry would show it]~~ **ANSWERED 2026-10-16 (re-answers25):** Reproduce the wrap (MATCHES). Insert and remove are symmetric (`0x0069E295` increment, `0x0069E29B` decrement, both after `GetConditionalBuildingStatus` `0x0069D8D0` finds the entry), so a wrap needs the entry to appear between the insert and the removal; shipped flows create a reward's entry (advisor reward evaluation) before it can be placed. Buildings loaded with a city are inserted before `Init` subscribes and are not re-counted; the saved count includes them, so their removal is correct. civic-buildings §4.3, test 11. In-game check: breakpoint at `0x0069E29B`, read the u16 at [entry+2] before the decrement; 0 means a wrap.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CB5:** The income line is keyed by the building type, not by the item's line. The line that
  `CreateLineItemForBuildingType` returns is used without a null check. [keep the keying. *[choice]* FreeCity4
  skips the income when creation fails, which the shipped budget never does for a new type]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice harmless). `CreateLineItem` always creates and stores a new item and returns it (budget.md, the department budget methods), so the null case does not arise in the original apart from an allocation failure.

## From the commercial, industrial, civic and park cores (filed by the coordinator from the `cores10b` lane's report, 2026-10-14)

cores10b implemented the four cores (merged `ab3c1c3`, ABI 47). Every listed spec test is a unit test and a
conformance vector (`ci13_*`, `cb12_*`, `pk12_*`). The 421-save corpus tests (commercial-industrial §13.9, civic §12.10,
parks §12.8) wait for a glue lane's save sweep, which the coordinator runs.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB6:** civic `Read` order: inserting the file's entries in order, as STLport does, would reverse each bucket chain,
  so §12 test 9 could not be byte-identical. [grow the table first, then insert in reverse; a key already present, or
  repeated in the file, keeps its first value]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. The original inserts in **file order**, each at the head of its chain (set `0x004F0940` → `0x00624DD0`, map `0x0069E720` → `0x0069E5F0`; `resize(n + 1)` before each insert changes nothing below 194 entries), so `Read` **reverses** every chain of two or more entries, and write → read → write is not byte-identical for such chains; a second round trip restores them. A repeated or already present status key **overwrites** the stored value (last value wins, `0x0069E944`–`0x0069E96D`); a repeated exclusion ID is a no-op. civic §7, and §12 tests 9 and 10 are corrected to expect the reversal (test 10: corpus re-write = the record with multi-entry chains reversed; a second round trip byte-identical).

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB7:** civic `Release` (slot 2). [sibling rule: above 1 it decrements; otherwise Shutdown, destroy, return 0]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. `Release` (`0x0069D7A0`) decrements above 1; at 1 or less it calls the deleting destructor (`0x0069E700`, destructor `0x0069E500`) **without** `Shutdown` and returns 0. The destructor releases the Landmarks department but not the list entries. The city always shuts the object down before its last release, so the bracket (Shutdown, then destroy) agrees in the shipped game and is accepted as a *[choice]*. civic §8.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB8:** civic null list entries. [the monthly pass skips them; an ape escape that draws one does nothing]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: the monthly pass dereferences each `maintained` entry without a test (`0x0069E0F0`–`0x0069E10B`), and the ape escape passes a null draw on to `AddControllersFromOccupantGroup` (`0x00517150` → `0x005163D0`) untested (the controllers' handling of a null target was not followed). The bracket is accepted as a *[choice]*. civic §4.6.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB9:** a business-deal income line that cannot be created (CB5). [skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: `SetType(1)` is called on a line that could not be created (`0x0069DF6D`–`0x0069DF78`). Skipping is a *[choice]*. civic §4.4.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB10:** a civic department that cannot be created in `Init`. [no name or maximum is set and nothing is held]~~ **ANSWERED 2026-10-15 (re-answers10):** partly. A Landmarks department that cannot be created crashes the original (name set on null, `0x0069DC69`); Building Maintenance and Business Deals are null-tested (`0x0069DCAB`, `0x0069DCF1`) and set nothing: MATCHES for those, a *[choice]* for Landmarks. civic §4.1.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB11:** a civic department whose funding cannot be read in the monthly pass. [that entry is skipped]~~ **ANSWERED 2026-10-15 (re-answers10):** the original crashes: it reads through the funding pointer without a test (`0x0069E146`). Skipping the entry is a *[choice]*. civic §4.6.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB12:** civic `DoBuildingRemoved` step 3. [releases only when a matching entry was found and erased]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES (`0x0069DBEB`–`0x0069DC02`): the release happens only when a matching entry was found and erased. civic §4.5.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB13:** civic `Read`. [atomic; the old Landmarks department is released, the old list entries are not]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES except atomicity. The old Landmarks department is released before its reference is read (`0x0069E9A1`–`0x0069E9AB`); the lists are cleared without releasing their old entries (`0x005DB3C0`). The original is not atomic (the containers fill entry by entry and the stream error is tested at the end); atomicity is a *[choice]*. civic §7.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CB14:** the civic temporary generators' counter n is not saved. [seed derive(S, 0x0A413481 ^ n), n counting the
  generators this object has made]~~ **ANSWERED 2026-10-15 (re-answers10):** accepted (glue, nothing in the original: it seeds each temporary from the clock). `n` counts the generators the simulator object has made, from 0 at construction, not saved. foundations §6.3's "in this session" is reworded to "by the owning object"; civic §5.

## From the commercial, industrial, civic and park glue (filed by the coordinator from the `cmglue` lane's report, 2026-10-15)

About `docs/specs/commercial-industrial.md`, `civic-buildings.md` and `parks.md` and the replacements
`cFC4CommercialSimulator` and `cFC4IndustrialSimulator` (`enable = commercial, industrial`),
`cFC4CivicBuildingSimulator` (`enable = civic`) and `cFC4ParkManager` (`enable = parks`) with their shadows
(`commercial_shadow = 1`, `civic_shadow = 1`, `parks_shadow = 1`); merged as `58b6d11`, ABI 53. The lane numbered
these CIV-1..CIV-6, PKG-1, PKG-2 and SWEEP-1. CIV-4 is the CB6 answer, applied: civic `Read` inserts in file order at
the head of each chain (a repeated exclusion is a no-op, a repeated status key's last value wins in place), conformance
`cb12_09` and `cb12_10` follow it, and the civic shadow pre-reverses the chains of the original's captured record. The
current answer is in brackets.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**CB15:** Lua `civic.get_landmark_count` has no published registry node (the budget and aura glues stub
  `register_lua` too). [logged as not published (stub)]~~ **ANSWERED 2026-10-15 (re-answers12):** **DIFFERS** (civic-buildings.md §4.1 step 6). The original publishes a game-data registry node, as the budget does: node `"civic"` under the root (registry slot 5, key = the simulator) and a call entry `"get_landmark_count"` (slot 3, key and callback `0x0069D740`, flags 1, context = the simulator). The callback writes `GetLandmarkCount()` as a Uint32 variant into output parameter ID 1 (`cIGZCommandParameterSet` slot 7). `Shutdown` removes the node (slot 7). The stub can become this recipe once the CB16 rows exist.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**CB16:** `docs/abi` lists only slots 3, 5 and 6 of `cIGZCommandParameterSet`, so a Lua getter cannot append its
  output parameter. [not published until the rows exist]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered (rows in `build-runs/re-answers12-abi-rows.md`; `docs/abi/vtables.tsv` already lists the whole vtable `0x00AD16C8`). The two slots a getter needs: 7 `SetParameter(uint32 id, const cIGZVariant&)` (`0x0092ABEB`, pop 8; find-or-insert by ID, then assign) and 8 `AppendParameter(const cIGZVariant&)` (`0x00929E19`, pop 4; ID = last ID + 1, or 0 when empty). The civic callback uses slot 7 with ID 1 (civic-buildings.md §4.1).

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**CB17:** civic slot 7 `UpdateConditionalBuildingStatus`: the spec does not say how to read the caller's tooltip.
  [read as a `cRZString`-layout begin/end pair; the status is copied, never written back]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (civic-buildings.md §4.7). The source struct is read in place: byte `+0`, u16 `+2`, and the `cRZString` at `+4` whose STLport begin / end pointers are at `+8` / `+0xC`; the range is assigned unless it is the same string (`0x0069EA1D`). Nothing is written back.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**CB18:** civic `Write` returns true when the stream refuses strings (`SetGZStr` false), so a stream without string
  support writes a short record with success. Is that the original's behaviour? [as the spec says: the result is the
  stream error state only]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (civic-buildings.md §7). `Write` (`0x0069D920`) ignores every stream call's result, `SetGZStr` included (`0x0069DA1E`), and returns `!stream.GetError()` (stream slot 18, `0x0069DA88`). Whether a stream that refuses strings also sets its error flag is the stream's own behaviour.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**CB19:** the in-memory layout of the 193-bucket status map beyond the bucket order (node allocation, erase) is not
  specified. [a game-layout hash map whose values keep a fixed address, merged field by field with the core; the core
  keeps the 193 order]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (civic-buildings.md §2). Nothing outside the class touches the table; slot 6 hands out the node's value address. Layout for reference: table at `+0x2C`, buckets at `+0x30`/`+0x34`, count at `+0x3C`; 0x20-byte nodes from allocator B (`+0` next, `+4` key, `+8` status, `+0xA` count, `+0xC` `cRZString`), so a value's address never moves while its node lives.
