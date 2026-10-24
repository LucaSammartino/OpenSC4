# landfill: answered questions (archive)
<!-- Archive for [landfill.md](../../landfill.md). Lanes do not read this file. Open questions are in ../landfill.md. -->

<!-- from: verify-landfill-records4 (2026-10-22) -->
- **LG31 - sections 1 (Owned objects), 4.4.9, 6.2, 7.1, 7.4 and 9: published native zone verification boundary:** Is there an already-published public-interface route for independently invoking native `cSC4LandfillZone` queries, mutations and record Read/Write on identical controlled inputs? `GetLandfillZoneFromCell` returns a borrowed pointer (no AddRef); the only published zone vtable contract is QI for `1` / `0x452294AA`, AddRef, Release, a no-op DoMessage, and a destructor slot. Section 7.4 explicitly says this is not a public interface, and section 1 says there is no GZCOM CLSID. The behavior and inline record format are specified, but no callable public zone methods or zone serializable interface are published; the ABI tables contain no zone rows. Please identify an existing permitted acquisition/calling/lifetime contract, or confirm that independent native zone-record verification remains unsupported through public interfaces. This asks for a published-contract decision, not new RE. [Park this verification task at its required initial boundary check. Do not call code addresses, inspect private fields, use developer serialization as a zone-method oracle, or count public occupant/developer checks as zone verification. No native comparisons or coverage credit claimed.]
  **ANSWERED 2026-10-24 (re-landfill-lg31):** MATCHES — independent native zone queries, mutations and record verification remain unsupported through the published public interfaces. Landfill §7.4 now states the boundary: slot 12 gives a borrowed pointer, balanced AddRef/Release permits retention only, and neither the known identity/message prefix nor inline developer persistence supplies a callable zone-method or zone-serializable contract. Sections 4.4.9, 6.2 and 9 remain behavior/layout requirements, not public acquisition/calling routes. Section 11.1 adds four admission/accounting vectors. The question is closed; independent zone verification stays parked, landfill verification stays partial, and no native comparisons or zone coverage credit are claimed.

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**LF1:** which of `AttemptAreaLevelling`'s two bools is the landfill's second `true`? Test 22 cannot tell them
  apart. [passed as `waterFlag`]~~ **ANSWERED 2026-10-14** from the exe: the bracket matches. The third argument
  (`withWater`) feeds the vertex average and the slope test; the fourth (`waterFlag`) is `ModifyTerrainHeight`'s
  second bool (`0x006C44F0`). Both are literal 1 (`0x006BA785`/`0x006BA787`), so the order is unobservable. Also:
  this path passes the **unclamped** rect (landfill.md §4.5.3).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**LF2 — zone `Init`:** is "levelAltitude is exactly 0.0" an exact float compare? [−0.0 counts as zero; NaN does
  not]~~ **ANSWERED 2026-10-14** from the exe: the bracket matches. It is `FUCOMPP` against +0.0 with `TEST AH,
  0x44` / `JP` (`0x006BB6E2`–`0x006BB6F2`): only an ordered equal compare recomputes. The centre cell is the C
  integer mean of the rect corners (landfill.md §4.4.9).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**LF3:** in what order does Shutdown release the message server and the city? [one `release_services` call after
  the zones shut down]~~ **ANSWERED 2026-10-14** from the exe (`0x006B94B0`). The original order is: unsubscribe
  and release the message server **first**; then shut down and release the zones; then release the department,
  budget, pollution, terrain and city; then `RemoveAgent` through the global city. The bracket differs, but zone
  `Shutdown` uses only its own references, so the difference is unobservable. Matching the original is recommended
  (landfill.md §1.2, §4.4.9).

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**LF4 — §4.7.5:** in what ring order does the zot spiral run? [clockwise from each ring's top-left corner; the
  second half of `lf11_11` depends on it]~~ **ANSWERED 2026-10-14** from the exe (`0x006BB3B0`–`0x006BB4DA`): **the
  bracket does not match.** The zot uses the plumbing §7.2 `SpiralIterator` mode over the zone rect. Ring r starts
  at `(cx + r, cz − r + 1)`, runs +z, −x, −z, +x, and ends at `(cx + r, cz − r)`, so ring 1 starts at `(cx+1, cz)`,
  not `(cx−1, cz−1)`. Out-of-rect cells are skipped one by one. landfill.md §4.7.5 has two worked examples, and
  test 11 now cites them. `lf11_11` must be regenerated.

<!-- from: From the landfill tests and the neighbours core (2026-10-14) -->
- ~~**LF5 — §4.4.4 step 3:** `LevelRect((x, z, x+1, z+1))` per cell is a 2×2 block if the rectangle is inclusive. Is
  that intended? [passed as written]~~ **ANSWERED 2026-10-14** from the exe (`0x006BC150`): the bracket matches. The
  rect is inclusive, so each region cell averages and levels the 2×2-cell block (vertices `x..x+2 × z..z+2`):
  `ModifyTerrainHeight(1, 1, x, z, x+2, z+2, avg, null, 5, null)`. The levelling is chained cell by cell (x outer,
  z inner) and reaches one cell past the region's +x and +z edges. Whether that was the author's intent cannot be
  known; it must be reproduced. Worked example in landfill.md §4.4.4 step 3.

## From the landfill developer glue and shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/landfill.md` and its `docs/abi` rows. Ini: `enable = landfill` (ten `landfill_*` bisection
switches); shadow `landfill_shadow = 1` (the original's SimNewMonth wrapped; its record read through its own Write
before and after). The glue's current answer is in brackets.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG1 — §4.4.5, §4.8.1:** what arguments does RemoveOccupant take for flora and base-texture removal? [(true, 3)]~~
  **ANSWERED 2026-10-14** from the exe (`0x006BC6D8`, `0x006BCA14`; `RemoveOccupant` `0x0049F240`): both are
  `(occupant, true, 3)`. The occupant is shut down after removal, and data4 of the posted `0x99EF1143` is 3. Only a
  base texture with `GetUsage() = 1` is removed. **The bracket matches.** Written into landfill.md §4.8.4.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG2:** how is the base-texture occupant created and given the landfill texture (SetEmptyCellTexture)? [not done]~~
  **ANSWERED 2026-10-14** from the exe (`0x006BA470`) and the saves. It calls GZCOM `CreateInstance(0xC97F987C,
  0x097F6D98)`, then Init, `SetUsage(1)`, `SetTextureSpecification` with one record (texture `0x1005`, the cell,
  `00 00 FF FF FF <zone display alpha> 01 00`, no random draw) and `InsertOccupant(occ, 0)`. All 94 usage-1
  textures in the local saves match. **"Not done" does not match**: the texture is saved, and §4.4.5 tests for it.
  Written into landfill.md §4.8.5.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG3:** how is the tile occupant created; is the SDK's `SetTileID` (slot 6) enough to attach the tile exemplar?
  [tiles live in FreeCity4 only; pollution's active-landfill flag is still set, which is what the simulation reads]~~
  **ANSWERED 2026-10-14** from the exe (`0x006BC7CF`–`0x006BC92A`, `SetTileID` `0x00477450`). **No.** `SetTileID`
  only renames the key of an exemplar that is already attached, and crashes when none is. The original loads the
  exemplar with `GetResource` and attaches it with `cISCExemplarPropertyHolder::SetDefaultExemplar` (QI
  `0x0AC2B5F7`). Landfill.md §4.8.1 gives the order and a GZCOM recipe. **The bracket does not fully match**: the
  occupants are also saved, block flora, are tested by `PlaceZone` and are drawn. Written into landfill.md §4.8.1.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG4:** who owns the reference in occupant manager slot 33's out-parameter? [slot 41 lists used instead]~~
  **ANSWERED 2026-10-14** from the exe (`0x0049DCE0` → `0x0049E690`, `AddRef` at `0x0049E7DD`; released by the
  landfill at `0x006BA454` and `0x006BC7AB`): **the caller**. A hit is `AddRef`ed; a miss leaves `*out` unwritten.
  **The bracket is equivalent** for presence tests if the slot 41 lists are released through slot 48. Written into
  landfill.md §4.8.6.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG5 — open question 6:** where does the zot orientation come from? [0]~~ **ANSWERED 2026-10-14** from the exe
  (`0x006BB510`–`0x006BB524`; `SC4ViewUtilities` `+0x10`, set from the camera rotation): it is the **current view
  rotation** (0 to 3) at placement, and it is saved in the prop record. **The bracket matches** while the view is at
  rotation 0. All 66 zot props in the local saves have 0. Written into landfill.md §4.7.5. The same answer closes
  lot-developer.md §12.2 Q3.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG6 — §3.3, foundations §13.4:** must the month run after pollution and budget? [not enforced]~~ **ANSWERED
  2026-10-14** (foundations §13.3/§13.4; the data links of §3.3): **yes**, *[compat]*. Otherwise each landfill
  intake reaches pollution's history and line 20 one month early, and each line 30 maintenance reaches the budget's
  settlement one month early. Landfill.md §3.3 has a worked example. **"Not enforced" does not match** whenever the
  landfill agent ends up before pollution or budget, which registration order does. The §13.4 deferral ends now that the
  landfill glue has landed. Written into landfill.md §3.3.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG7:** does FindNetworkTile's return carry a reference? [borrowed]~~ **ANSWERED 2026-10-14** from the exe
  (`0x004B0B20`; `PopulateZone` `0x006BCDF6`–`0x006BCE16`): **borrowed**. It makes no `AddRef` call, and
  `PopulateZone` only tests the result for null. The map is fetched each time from the global city's slot 40.
  **The bracket matches.** Written into landfill.md §4.4.5.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG8:** tiles are not saved; they are rebuilt at PostCityInit (interior orientation 0, no random draw). [choice]~~
  **ANSWERED 2026-10-14** from the exe (PostCityInit, §3.2) and the saves. The original **saves** each tile (class
  `0x8A22BEBE`, with tile, orientation and exemplar key) and **does not rebuild** on load. There is no draw. In the
  three saved landfills, all 142 cells match the §4.8 rules for the saved heaps, and the interiors have orientations
  0/2/4/6. **The choice matches** for loading original saves, except that interior orientations become 0. It
  **does not match** for a FreeCity4 save loaded by the original: with no tiles, maintenance and the terrain dezoning
  differ. Written into landfill.md §6.4.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG9:** is a null `GetLandfillZoneFromCell` acceptable to the city situation manager? [null]~~ **ANSWERED
  2026-10-14** from the exe (`SetTarget` `0x00528AB9`–`0x00528C0A`, `GetDistanceToTarget` `0x00526A20`). **Yes.** A
  null zone makes both functions fall back to the target's lot. A tile has none, so there is no zone marker, and the
  distance is measured to the tile. The original returns null itself for a zone's last row and column, and the effect
  is UI or mission only. **The bracket matches.** Correction: the zone rect is read as integers, not floats, so
  landfill.md §10.1 quirk 15 is withdrawn. Written into landfill.md §9.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG10:** city slot 97 returns a float in ST0 (`docs/abi`) where the spec says "—". Which is right? [discarded]~~
  **ANSWERED 2026-10-14** from the exe (`0x00459440`): **the spec**. Both results are stored with `FSTP` through the out
  pointers, and the x87 stack is empty at `ret 0x10`. The docs/abi row has said this since re-answers2 PR6 (`7d2f368`:
  "returns nothing"). **The bracket is harmless but not exact.** If the call is declared as returning a float, every
  discard pops an empty x87 register. That is a masked stack underflow: it sets the sticky IE and SF status flags
  (it would trap if IE were unmasked) and stores no value. Using the "result" would read the NaN indefinite. Declare
  the slot `void`. Written into landfill.md §4.2 and §8.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG11:** should the tile item cost be read from the tile exemplar? [no]~~ **ANSWERED 2026-10-14** from the exe
  (`getLandfillTileCost` `0x006BA3C0`). **Yes.** It reads `0xEA54D286` through the tile occupant's property holder,
  and its only source is the attached tile exemplar. `kDefaultLandfillTileCost` is used only where a cell has no
  tile, or the property is missing. **The bracket matches only if the glue's fixed costs equal the shipped tile
  exemplars** (10/20/20/30/40). Using the default for every cell gives, for example, 45 instead of 160 for a full
  3 × 3 zone. Written into landfill.md §4.9.1.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG12:** should the shadow clamp GetGarbageSupply? [live plus delta, clamped]~~ **ANSWERED 2026-10-14** (landfill
  §4.7.1, pollution §8.1 slot 29). This is shadow glue: **no original behaviour; the implementer's choice stands.** A
  fact that makes the clamp moot: in the original month the intakes sum to at most `t ≤ supply`, so no
  `AddToGarbageSupply(−n)` clamps, and "live plus delta" is exact. The clamp matters only for a negative supply from a
  damaged save, which `GetGarbageSupply` itself clamps and stores. Written into landfill.md §4.7.1.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG13:** on Shutdown, does a zone release its zot or also remove the prop? [release only]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006BB766`–`0x006BB798`): it **removes the prop too**. It calls `RemovePropA` through the city's prop
  manager when there is one, then releases the zot. **The bracket does not match**, but the difference is unobservable
  in the shipped flows: every `RemoveZone` dezones cells, which already removes the zot. Only the city-close shutdown
  still holds one. Written into landfill.md §4.4.9.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG14:** the shadow's supply is live plus delta, clamped. [choice]~~ **ANSWERED 2026-10-14** together with LG12
  (landfill.md §4.7.1). This is shadow glue: **no original behaviour; the implementer's choice stands.** In the
  original month the supply ends at exactly `s − Σ n`, and no clamp engages, so "live plus delta" equals the
  original and the clamp is inert. The only exception is a negative supply from a damaged save.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG15 — §1.1:** only the city and the message server are held; terrain, pollution and budget are fetched per
  call. [as stated]~~ **ANSWERED 2026-10-14** from the exe (`Init` `0x006B9D98`–`0x006B9E87`). The original holds
  **five** services, all with `AddRef` from the global service pointers: city, terrain, pollution, budget and message
  server. It does not hold the simulator. **The bracket is acceptable**: only reference counts differ, because each
  per-call fetch names the same object for the city's life. Written into landfill.md §1.1.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG16:** the 3D view level is not reachable through a documented interface. [not done]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006BA07F`–`0x006BA0D5`; `SL::OnView3DCreate` `0x00602120`). The original uses the 3D view global
  `0x00B43DC4`, slot 22, with no null check. The same view **is** reachable through public interfaces: app main window,
  then child `0x6104489A`, then `GetChildAs(0x9A47B417, cISC4View3DWin 0xFA47B3F9)`, as the SDK's
  `SC4UI::GetView3DWin()` does. **"Not done" does not match** (the seagulls never toggle), but the effect is rendering
  only. Written into landfill.md §3.2.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG17:** the traffic network map comes through city slot 40 (`GetTrafficNetwork`, `intptr_t` in the SDK). Right?~~
  **ANSWERED 2026-10-14** from the exe (`0x004596F0`; created at `0x0045B07D`; used at `0x006BCDF6`). **Yes.** Slot 40
  returns the city's `cISC4TrafficNetworkMap*`: the interface pointer created with IID `0xCA0F81B9`, **borrowed**, with
  no `AddRef`. `PopulateZone` calls its slot 8 directly, with no QI and no release. Written into landfill.md §4.4.5
  and §8.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG18 — §1.1/§1.2:** which simulator does the original use for AddAgent before services are acquired? [the app's
  city's simulator, held until RemoveAgent]~~ **ANSWERED 2026-10-14** from the exe (`Init` `0x006B9D98`–`0x006B9DBA`;
  `Shutdown` `0x006B95D3`–`0x006B95ED`; `SL::OnCityInit` `0x00601C20`). The **city's simulator** (city slot 50), read
  from the cache global `0x00B43D5C` that `SL::OnCityInit` fills. It is not held. `RemoveAgent` fetches it again from
  the global city. **The bracket matches**: it is the same object, and only the hold adds a reference count. Written
  into landfill.md §1.1.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG19 — foundations §6/§12:** the core uses its own CRT-style generator per object (state 1), not the game's
  unseeded per-thread `rand()` sequence. Acceptable?~~ **ANSWERED 2026-10-14** from the exe (`UpdateSurroundingTiles`
  `0x006BCFC0`; `rand` `0x009EFB9F` at `0x006BD201`, `0x006BD271`, `0x006BD2DD` and `0x006BD34B`). **Acceptable; it
  is foundations §12.6's decided choice.** The original draws from the per-thread CRT sequence, which every
  simulation-thread caller shares, so the values cannot be reproduced. The count (one draw per mask-15 cell) and the
  formula `2 × (rand() mod 4)` are what must match. Written into landfill.md §4.8.

<!-- from: From the landfill developer glue and shadow (2026-10-14) -->
- ~~**LG20:** a message from another thread that cannot get the lock is dropped. Acceptable?~~ **ANSWERED 2026-10-14**
  from the exe (`DoMessage` `0x006B9EA0`) and foundations §18.2. The lock is glue: **no original behaviour; the
  implementer's choice stands.** The original takes no lock and handles every message, and all of them arrive on the
  main thread, so it never drops one. Dropping is therefore harmless only while it never happens. A dropped zoning or
  terrain message would change the zone, so drops should be logged, and waiting is safer. Written into landfill.md
  §3.2.
- Not done (each logged once): tile models, `SetEmptyCellTexture`, zone effects (trucks, seagulls), zot
  orientation, the 3D view level, `GetLandfillZoneFromCell`. The shadow cannot compare the zots, trucks and seagulls
  the original drew, the tile occupants it changed, the expenses it booked or the history it sent.

<!-- from: From applying the pathfinder, zones, landfill and neighbours answers (2026-10-14) -->
- ~~**LF6 — LF3:** should the core add the message-server release callback (an ABI bump) to match the original's
  shutdown order? [no: one `release_services` call after the zones shut down; LF3 says the difference cannot be
  observed]~~ **ANSWERED 2026-10-14** from the exe (message server global 0x00B43CCC written only in `SL::OnAppInit1`
  at 0x006024BB) and written into landfill §1.2.
  - No original behaviour; the implementer's choice stands. The message server is the framework's application-lifetime
    service, so the developer's release of it is never the last one and its timing cannot be observed. No ABI bump is
    needed.
  - What could be observed is the unsubscription, which the original does before the zones shut down. One
    `release_services` call afterwards is equivalent while no message reaches the developer in between, which holds
    (LF3).

## From applying the power, plumbing and landfill answers (filed by the coordinator from the `apply6b` lane's report, 2026-10-14)

PW1–PW8, PS1–PS8 and LG1–LG20 are applied or confirmed, except LG5 (orientation stays 0; there is no documented route
to the view rotation) and LG16 (seagull effects are not implemented). LG1 and LG2 need an in-game check. The current
answer is in brackets.

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**LG21:** LG5: is there a documented route to SC4ViewUtilities `+0x10`, for example a docs/abi row for
  `cISC43DRender` slot 8? [orientation 0]~~
  **ANSWERED 2026-10-14** from the exe (`SL::OnView3DInit` `0x00602590`; `0x0043FD00`; `0x007C9FB0`) and written
  into landfill.md §4.7.5 and §8. **Yes, there is a route.** The global `0x00B43DD8` is filled from the city view as
  `cISC4View3DWin` (IID `0xFA47B3F9`) slot 4 `GetRenderer`, then `cISC43DRender` slot 8 `GetViewUtilities`. Both
  are pop 0, return EAX and give a borrowed pointer. The rotation is the 32-bit integer at `+0x10` of the view
  utilities object, which is not a GZCOM object. Neither interface has a rotation getter, so that field read is
  the only route. **The bracket (orientation 0) matches only at the first rotation.** *[choice]*: glue reads the
  rotation through this route when it places the zot, and uses 0 when there is no view. ABI rows are in the
  re-answers9 report.

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**LG22:** Landfill §4.8.5: is the in-memory record that SetTextureSpecification takes laid out in the saved 14-byte
  order, and what is its stride? [saved order; one record in a 16-byte buffer with zeroed padding]~~
  **ANSWERED 2026-10-14** from the exe (`SetTextureSpecification` `0x005E0B20`, `Write` `0x005DF1E0`,
  `SetEmptyCellTexture` `0x006BA470`) and written into landfill.md §4.8.5. **Yes: the saved order, with a
  16-byte stride.** The texture is at `+0x0`, x and z at `+4` and `+5`, orientation at `+6`, the configuration
  byte at `+7`, the five unnamed bytes at `+8`–`+0xC`, the frame at `+0xD`, and padding at `+0xE`–`+0xF`. The
  padding is copied into the vector but is never read or saved. The original's own record leaves it unset (stack
  garbage). **The bracket matches.**

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**LG23:** LG8: a FreeCity4 save made while tiles were kept in FreeCity4 has no game tiles. [PostCityInit rebuilds
  only the cells without a tile]~~
  **ANSWERED 2026-10-14** from landfill.md §6.4 (LG8: the original saves its tiles and never rebuilds them on load)
  and written into landfill.md §6.4. **No original behaviour exists for this case**, so the implementer's choice
  stands. The original loading such a save shows no tiles until each heap next changes. **The bracket is
  acceptable.** Define the cells to rebuild as those whose rule gives a tile other than `0x02F10000` and where
  slot 33 finds no landfill occupant. On an original save that rebuilds nothing. The one exception is a cell left
  without a tile by `AddRegionToZone` (§4.4.4 step 4), which the original fills only when its heaps next change.

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**LG24:** In the original, a zone that was never initialised never releases its zot at Shutdown. [we release our
  reference at the developer's Shutdown, without RemovePropA]~~
  **ANSWERED 2026-10-14** from the exe (developer `Shutdown` `0x006B94B0`, zone `FinalRelease` `0x006BB8C0`, zone
  destructor `0x006BBBC0`) and written into landfill.md §4.4.9. **The premise is incomplete.** The zone's `Shutdown`
  skips the zot, but the developer's `Shutdown` then releases the zone. Its last reference runs the destructor,
  which releases the zot without `RemovePropA` and without nulling the field. **The bracket matches.**

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**LG25:** `cISCExemplarPropertyHolder` has only one docs/abi row (slot 3); slot 4 `GetDefaultExemplar` has none.
  [a reviewed slot count of 5 against 4; only slot 3 is called]~~
  **ANSWERED 2026-10-14** from the exe (vtables `0x00A8CD08` and `0x00A871A0`) and written into landfill.md §4.8.1
  and §8. **The vtable has five slots**: 0 QI thunk, 1 `AddRef` thunk (`0x005E06A0`), 2 `Release` thunk
  (`0x005E06B0`), 3 `SetDefaultExemplar` (`0x00458E60`, pop 4, void) and 4 `GetDefaultExemplar` (`0x0060CE80`,
  pop 0, EAX, borrowed). A zero dword follows at `0x00A8CD1C`, then the next vtable. **The reviewed count of 5 is
  right.** A declaration that stops before slot 4 is safe only for calling. The landfill calls only slot 3. The
  missing rows (1, 2) are in the re-answers9 report; main's interfaces.tsv already has slots 0, 3 and 4.

<!-- from: From applying the power, plumbing and landfill answers (2026-10-14) -->
- ~~**LG26:** LG3: when InsertOccupant fails after Init, should the occupant be shut down? [released without Shutdown;
  the tile is kept in FreeCity4]~~
  **ANSWERED 2026-10-14** from the exe (callers `0x006BD234`/`0x006BD2A4`/`0x006BD310`/`0x006BD37E`;
  `cSC4Occupant::Shutdown` `0x005E8980`; `Release` `0x005E8DF0`) and written into landfill.md §4.8.1. **No
  Shutdown.** For a new tile the insert cannot fail, and no caller tests the result. Had it failed, the tile would
  be freed when the queued `0x99EF1140` message released it, without `Shutdown` or its `0x99EF1141`. The original
  keeps no tile list, so its later cell lookups would find no tile. **"Released without Shutdown" matches.** "Kept
  in FreeCity4" has no counterpart. *[choice]*: keep it and log the failure.

## From the apply6c follow-up (landfill game tiles; filed by the coordinator, 2026-10-14)

Regress r6b (run 100): every LG3 game tile failed to insert (`gameTiles=0 gameTileFailures=12`). apply6c logs each
step and adds a fallback. The current answer is in brackets.

<!-- from: From the apply6c follow-up (2026-10-14) -->
- ~~**LG27:** Does the landfill occupant's QueryInterface (`0x00477130`) answer `0x0AC2B5F7`? If not, which object
  does: the sub-object at +0x20, or the occupant's property holder? [QI on the occupant, then on its property holder]~~
  **ANSWERED 2026-10-14** from the exe (QueryInterface `0x00477130` and its thunks `0x00477210`, `0x00477230`,
  `0x00477240`, `0x00477260`; `CreateLandfillOccupant` `0x006BC845`–`0x006BC84C`; `cSC4Occupant` constructor
  `0x005E9220`) and written into landfill §4.8.1. No, and no other object of the tile does: every sub-object's QI is a
  thunk to `0x00477130`, which answers only `0x2A15F262`, `1`, `0x26BB767E` and `0xE4FDA3D4`. The original never asks.
  It calls `SetDefaultExemplar` (slot 3) directly on the sub-object at +0x20, after the constructor and before `Init`
  and the insert. The bracket cannot work, as run 103 shows. *[choice]* Use the holder at `cISC4LandfillOccupant*` +
  0x20 (= `AsPropertyHolder()` + 0x14), or the layout-free `Read` route in §4.8.1.

<!-- from: From the apply6c follow-up (2026-10-14) -->
- ~~**LG28:** Does the GZCOM default constructor `0x004774C0` set type `0x6A11BD2B`, as the tile constructor
  `0x00477270` does? If not, does InsertOccupant accept the type it leaves? [left as the constructor sets it; the
  type is logged]~~
  **ANSWERED 2026-10-14** from the exe (default constructor `0x004774C0`, type stored at `0x00477553`; `cSC4Occupant`
  constructor flag byte at `0x005E9265`) and written into landfill §2.3 and §4.8.1. Yes: it sets type `0x6A11BD2B` like
  the tile constructor. Flag 4 is set by the base constructor for every occupant, so the two constructors differ only in
  tile and orientation (the spec's "flag 4 not set" was wrong and is corrected). InsertOccupant never reads the type
  (LG29). The bracket matches.

<!-- from: From the apply6c follow-up (2026-10-14) -->
- ~~**LG29:** When does InsertOccupant (`0x0049EF60`) return false: already inserted, not initialised, its box, or its
  type? [each failure is logged with those values]~~
  **ANSWERED 2026-10-14** from the exe (`InsertOccupant` `0x0049EF60`: flag test `0x0049EF8A`, holder test `0x0049EFBB`,
  first-cell duplicate scan `0x0049F158`–`0x0049F182`) and written into landfill §4.8.1. Only for a null occupant, an
  occupant whose flag 1 ("inserted") is already set, a null `AsPropertyHolder()` (never for game occupants), or an
  occupant already listed in its first cell. Never for the type, the initialised state (the insert calls `Init` itself)
  or the box. Logging these is the implementer's choice; the bracket stands.

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**LG30:** LG21 (landfill §4.7.5): should the view rotation be read when the zot is placed (what the glue does) or
  cached at view init, as the original's global is? [read at placement through the documented route; 0 without a
  view. Only rotation 0 has been seen in game: no script action rotates the view]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (landfill.md §4.7.5, "When the rotation is read"). The global caches the view-utilities object pointer, not the rotation; `AddZot` reads `+0x10` at placement (`0x006BB510`, `0x006BB51F`), so the original also uses the rotation current at placement. Without a view the original reads through a null pointer, and the glue's 0 is a *[choice]*.

<!-- from: verify-landfill-records4 occupant-only round 2 (2026-10-22) -->
- **LG32 - sections 2.3 and 7.3: inherited SetPosition return on an unregistered factory occupant:** What return contract, if any, applies to the SDK's `bool cISC4Occupant::SetPosition` for this class? Public native factory/serializable calls on a version-3 occupant with a null-parent, empty holder set position to `(88.5,-4,120.25)` and recomputed bounds exactly as Rust predicts, and all 57 written record bytes agree. Nevertheless the SDK bool reads false. SetOrientation(-123) returns true and same-ID SetTileID(0x02F10010) returns the previous ID exactly. Observed with turbo in `vlfr4-r2-on-reload-2032-20261022a` and confirmed at normal speed by `vlfr4-r2-normal-diagnostic-20261022b` (clean/ref0). Neither object is Init'ed or inserted. No private fields, code addresses or native-output seed into Rust were used. [Compare the specified position/bounds/record effects; log this return as unscored rather than inventing a true-return requirement. Rust's existing setter returns no value. Do not infer a general return contract or request new RE.]
  **ANSWERED 2026-10-23 (re-landfill-occupant-read4):** MATCHES — the measured effects and false SDK observation fit the concrete landfill body; keep the return unscored. **DIFFERS — the SDK Boolean declaration is not a success contract for this body.** Landfill §7.3 now specifies a void position operation: `0x00477600` copies f32 words and computes bounds, leaving EAX = input z bits (`0x0047760F`–`0x00477657`). For z = 120.25, EAX = `0x42F08000`, AL = 0. The shipped caller ignores the return (`0x006BC8F4`–`0x006BC8FB`). No registration branch and no rule inferred for other classes. Literal typed-output and residual-register vectors are in landfill-occupant-read-vectors.md §4; proposed S1 remains unmeasured. No implementation or verifier changes.

<!-- from: verify-landfill-records4 occupant-only round 2 (2026-10-22) -->
- **LG33 - section 6.4 and foundations 10.3: propagation of a failed nested holder Read:** Does the occupant/base reader ignore the exemplar holder's false return when the primitive stream remains error-free? An independently authored 57-byte input has outer version3, base version4, removal0x0102, highlight0, flags0xF4, type0x6A11BD2B, cell rect FF/00/00/00, exemplar-holder version changed from2 to1 at offset16, plain-holder version2, count0, null parent key, tile0x02F10010, orientation-7, position(-24.5,12.25,40), empty0. Public native Read returns true after consuming39/57 bytes with GetError=0; Rust returns false at the failed holder version. Outer/base wrong-version and final-byte truncation cases agree on rejection. Confirmed in both labels cited in LG32; normal speed is clean/ref0. [Keep Rust's existing strict nested-read failure propagation pending a published clarification; report this one malformed-input difference, and do not score subsequent native state/Write after divergent acceptance. No native output seeds Rust, no behavior changed to fit this measurement, and no new RE requested.]
  **ANSWERED 2026-10-23 (re-landfill-occupant-read4):** DIFFERS — strict propagation of exemplar rejection does not match occupant base `Read`. It ignores exemplar AL at `0x005E8DD8` and returns stream-error status at `0x005E8DE1`–`0x005E8DED`. **MATCHES — native true / 39 of 57 / error 0 follows from the published body.** Landfill tests base AL at `0x0047758D` then consumes a 21-byte tail from offset 18. Exemplar itself still tests plain AL (`0x005FF454`–`0x005FF456`); foundations §10.3 remains strict for that layer. Landfill §6.4 and lot-records §2.2–§2.3 now distinguish all four layers. The companion vectors specify proposed byte/state expectations and mark failures with unread stack temporaries **PARTIAL**, rather than inventing deterministic values or a bounded failed-count loop. The historical acceptance difference and agreed outer/base/final-byte rejection remain recorded; subsequent divergent native state remains unscored. LG31 and every unrelated unknown remain open. No implementation or verifier changes.
