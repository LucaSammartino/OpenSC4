# mysims: answered questions (archive)

<!-- from: glue-mysimdispatch-shadow9 (2026-10-24); imported from its reserved worktree because this base predates its question -->
- **MS20 — dispatch native observation/replay ABI (§4.6.1–4.6.2, §7.2.1):** publish the ordered slots, precise argument/return types and ownership for IID `0xEB8B6F9A` (Init/Shutdown, queue operations, state/flag setters, four location getters, clock/wait getter, GoTo/Enter/Exit and geometry/stop helpers), plus a public way to discover an inactive MySim's dispatch. Also identify how a read-only observer can receive every dispatch request input, and constructor-only defaults beyond state/previous=-1, flags/minutes=0 and travel=255. Do not guess slots, mutate live owners, infer requests from native outputs or label a native-record roundtrip independent verification.
  **ANSWERED 2026-10-24 (re-mysims-ms20):** DIFFERS — §7.2.3 publishes the entire nine-slot primary table: destructor/RemoveRef/GetRefCount occupy slots 3–5; only Can/Enter/ExitUDriveIt follow. The requested queue/clock/location helpers and Init/Shutdown are direct methods, not slots. There is no complete public read-only input observer or universal inactive-dispatch discovery API. Independent replay needs captured request/service/callback inputs; constructor records alone cannot verify live trips. Explicit constructor defaults exclude unwritten tuning +20 and leg indices. Three normative vectors in §12.29.

<!-- from: re-mysim-dispatch5 (2026-10-23); residual of MS17 -->
- **MS18 — multimodal route-ready handoffs (§4.6.1, §4.11, §7.2.1, §8):** complete the dispatch state-2 route-ready automaton in `switchTravel` (`0x00435840`): mode-to-mode handoffs (0/1/2/3/6/7/8/0xFE/0xFF), stop selection and neighbour retries, final-segment proximity geometry/rounding and equality boundaries, idle versus stuck recovery, full notice ordering including `0xEBA06BD0`, `0xEBA06BD4`, `0xEBA0732E`, `0x2BAC398F`, and any delegated automata/model RNG. Also close dispatch-tool input lifecycle and player-drive mission callbacks beyond the GoTo/Enter/Exit contracts now specified. MS17's queue, clock/wait timestamp, notification setters, five tuning field uses, request entry points and serialized payload are settled; the whole dispatch owner is **not** yet specified. [Do not infer visible motion, retry selection or complete-trip success from accepting a destination request.]
  **PARTIAL 2026-10-23 (re-mysim-ms5):** §4.6.2 settled asymmetric lot geometry, float32/truncation and equality boundaries, ordinary switch waits, first-match stop mapping, timestamps/notice order, strict expiry and common tail. Remaining work was the branch/mask matrix, neighbour retries, recovery notices, input lifecycle, mission callbacks and delegated RNG.
  **ANSWERED 2026-10-24 (re-mysims-ms20):** DIFFERS — §4.6.3–§4.6.5 complete the dispatch-side priority/mask decisions, same-mode idle fallback, FE/FF behavior, idle/stuck and callback ordering; §7.2.2 covers input/pylon lifecycle. The neighbour retry helper always fails because its candidate list is never populated. Near FE arrival writes duration, not deadline. No dispatch RNG is consumed; automata/model realization remains a foreign service input. §12.29 adds 18 normative MS18 cases (23 with §12.28), including all 42 unequal ordinary-mode pairs. These specifications are not live owner verification.

<!-- from: core-mysim-dispatch-close5 (2026-10-23) -->
- **MS19 — location comparison and named teleport (§4.6.1, §4.7.1, §7.2.1):** §4.6.1 and §6.2.1 cite an identity/cell equality rule in §4.7.1, but that section defines validity, assignment and coordinate arithmetic without equality precedence. Does occupant identity override cell differences, how are null/mixed occupants compared, and does a neighbour's regional city participate? Also specify the effects and notice ordering of `TeleportToCommuteLocation` called by DispatchCommute in MySim state 1: current/active/commute changes, validation failure, flags/state, deadline and return behavior. [Literal provisional equality compares `(occupant, cell)` and ignores timing/world coordinates; state-1 DispatchCommute explicitly returns UnsupportedMs19 without inventing teleport/arrival. Other published dispatch branches are implemented; MS18 still owns unresolved transport handoffs.]
  **ANSWERED 2026-10-23 (re-mysim-ms5):** DIFFERS — §4.7.1 requires occupant, both flag bytes, cell and regional coordinates all to match; world/timing fields are ignored. §7.2.1 specifies unconditional commute-to-current copy, travel FE, deadline now + duration, state 3 and flag 2, flags notice before state notice, no arrival/validation, void return. DispatchCommute's following home queue request remains successful. Five normative cases in §12.27.

- **MS17 — dispatch owner scope (§1.1, §3.3, §4.6, §4.11, §6.2, §7.2, §8, §10), answered parts:** the trip controller was assigned away and an immediate-arrival stub permitted, leaving its queue, clocks, notifications, tuning, request entry points and record unspecified.
  **ANSWERED 2026-10-23 (re-mysim-dispatch5):** DIFFERS — the dispatch belongs to this spec. New §4.6.1 specifies separate locations, queue, clock and settled transitions; §3.3 specifies synchronous owner delivery; §4.11 records the five tuning effects; §7.2.1 specifies LeaveForWork/ReturnHome, GoTo and player-control guards/exit. §6.2.1 publishes the u32-version-1 dispatch record, also used inline by MySim v12. The wait getter returns a stored switch timestamp, not elapsed wait. Covered operations consume no residence RNG; no RNG bytes are serialized. §10 removes permission for immediate-arrival stubs. Eight normative spec vectors added in §12. The remaining route-ready multimodal handoffs, complete dispatch-tool lifecycle and delegated automata RNG are explicitly **unanswered MS18**, not claimed complete.
<!-- Archive for [mysims.md](../../mysims.md). Lanes do not read this file. Open questions are in ../mysims.md. -->

## From the mysims core audit (cores26, 2026-10-20); answered by re-answers35

- **MS1 (§11.2.1, §6.2 rows 25 and 30):** the unnamed s32 pair (record row 25, version ≥ 17) and the unnamed bool
  (row 30, version ≥ 14) have no observed meaning. [inferred: the pair is the idle offset; the bool is UI state]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS — row 25 (MySim+0x204/+0x208) is dead data: zeroed in the constructor (0x00429A9E), written/read by Write/Read, and touched by no other cSC4MySim method (asm scan of all 69); row 30 (MySim+0x245) is the "pylon locked" flag set by DispatchToolLockPylon (0x0042728E) and gating is_pylon_location, DispatchToolEnd, the pylon timer and UpdateTripVariables. Neither is idle offset or UI state. Spec §2.2, §6.2 rows 25/30.

- **MS2 (§11.2.2, §4.2):** UI service `0xA417445E` (instance `0x5A4`) slot `+0xA8` — which window or state blocks
  the MySim daily and monthly steps? [inferred: a modal window]
  **ANSWERED 2026-10-21 (re-answers35):** MATCHES — service 0xA417445E is the window manager (kGZWinMgrDefaultSysServiceID, cIGZWinMgr IID 0x5A4); slot 42 (+0xA8) resolves to 0x009DB025, which returns (modal depth at WinMgr+0x17C) != 0, the counter cGZWinMgrBase::DoModalWin increments/decrements around its loop (0x009DBF11/0x009DBF2D). simulate_day (0x0042DCD9) and simulate_month both skip while any modal window runs. Spec §4.2.

- **MS3 (§11.2.3, §2.2):** does MySim `Init` re-resolve the neighbour job city from the saved cell (the app's
  region lookup), and does it set `neighbor_job` true only when a city is found? [inferred: yes]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS (in part) — yes, Init (0x0042CEF3-0x0042CF44, state != 0 only) re-resolves: if the MySim has no job and the saved neighbour cell is not (0,0), it calls cISC4App slot 35 then cISC4Region::GetCity(x, z) and stores a non-null result; there is no separate neighbor_job bool ("has neighbour job" = the city ref is non-null). What Init sets unconditionally in that branch is the commute-check flag, so the next day's CheckCommute validates the job. With a job or cell (0,0) the ref is released (quirk: cell (0,0) is never re-resolved). Spec §2.2.1.

- **MS4 (§11.2.4, §4.9):** the building development simulator's slot 44 list used by `DistanceToClosestSpecial`
  (§7.2): landmarks and rewards only, or every special building? [inferred: landmarks and rewards]
  **ANSWERED 2026-10-21 (re-answers35):** MATCHES — DistanceToClosestSpecial (0x00426AE0) walks cISC4City slot 53 -> BDS slot 44 (0x00520080, the list at BDS+0x4C), which BDS fills from occupant inserted/removed messages only for occupant groups 0x150A (landmark) and 0x150B (reward), previews excluded (building-development.md §3). Distance is float32 sqrt(dx^2+dz^2) truncated, unsigned minimum from 0xFFFFFFFF; entries without cISC4BuildingOccupant skipped. Spec §4.9.

- **MS5 (§11.2.5, §4.5):** the exact clamp and neighbourhood of `local_traffic_congestion` (the Mac build shows
  the rectangle grown by 1 cell on each side, clamped to `[0, CellCountX/Z − 1]`), and the neighbour-job constants
  of `job_condition` / `job_has_power` / `job_has_water` (0 / 1 / 1 in the core). [inferred]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS — local_traffic_congestion (0x0042B3DE-0x0042B581): home lot bounding rect grown by 1, min edges clamped to 0, BOTH max edges clamped to CellCountX-1 (city slot 93; quirk); only the border ring of the grown rect is visited (not the interior), and only cells with traffic GetCapacity(1,x,z) != 0.0 count; u8 max of the congestion map, 0 if none. Neighbour-job constants confirmed 1/0/1/1 (is_neighbor/condition/power/water). New: a city job whose building profile purpose is 5 (agriculture) gets job_has_water = 1 without asking the plumbing simulator (0x0042BA92). Spec §4.5.

- **MS6 (§11.2.6, §4.7):** does the traffic pathfinder `CheckCommute` and `FindNewJob` run have any side effect on
  traffic data (a recorded trip)? [inferred: no]
  **ANSWERED 2026-10-21 (re-answers35):** MATCHES — neither CheckCommute (0x0042D9E0; pathfinder slots 5, 9, 18, 19, 20, 23, 26, then Release) nor FindNewJob (dispatch SetCurrentCommuteDestination 0x004350D0 -> validateCommuteLocation 0x00432F50) calls ApplyTraffic (cISC4PathFinder slot 29, +0x74) or another volume writer: no trip is recorded. Only the process-wide work-check interval/adaptation counter (pathfinder.md §4.11) is shared. Spec §9.

- **MS7 (§11.2.7, §4.1, §4.6):** the "announce" check of `AnnounceMoveIn` (the class slot that passes on the
  reload case), and the dispatch state numbering 4/5/6. [inferred: announce is skipped after a load]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS — the "announce" check is cISC4MySim slot 56 RequestToPlaySound (0x00424DE0), a sound throttle, not a reload test: true when state != 0 and the MySim's seconds timer (cRZTimer units 5, MySim+0x2A0) is more than 4 s past the last true reading (MySim+0x29C, initially 0), which it then stores; only MySimMovedIn is gated, UpdateVariables and MYSIM_MOVE_IN always run. States: 4 to work, 5 home (not when state 1), 6 elsewhere/pylon, set on dispatch trip start 0x4BB71A7E (0x0042D364); arrival sets 2 (AnnounceMoveIn if it was 1) or 3. Spec §2.2, §4.1.

<!-- from: re-mysims-contracts4 (2026-10-21) -->
- **MS8 (§1.1, §4.7):** the job-search pathfinder's exact setup — mysims.md says "destination mask `0x1CF`" but
  `0x1CF` is a travel-type mask (Walk/Car/Bus/PassengerTrain/Subway/ElevatedTrain/Monorail) in traffic-simulator.md,
  not a destination bitmask; the core uses it as the travel-type mask with filter 3, flags 1|2, density 0. Which is
  it? [inferred: 0x1CF is the travel-type mask]
  **ANSWERED 2026-10-21 (re-mysims-contracts4):** DIFFERS — travel mask 0x1CF (allowed and starting), filter 0 (not 3), flags 1|0x10 (not 1|2), destination flags by wealth (1->0x1, 2->0x2, 3->0x4, else 0x1; OR-accumulated), density 1 (not 0), start rect = home lot rect; mysims.md §4.7.1
- **MS9 (§4.7):** does the dispatch re-run the pathfinder once per candidate lot until one succeeds, or a single
  FindPath with filter 0 (nearest standard destination) that picks the lot itself? [inferred: single run, filter 3]
  **ANSWERED 2026-10-21 (re-mysims-contracts4):** DIFFERS — a single FindPath with filter 0 picks the nearest lot or neighbour pool with free job capacity; no per-candidate loop; mysims.md §4.7.1
- **MS10 (§1.1, cSC4MySimPool):** the Sim-template record layout — where templates come from (installation files?
  which paths?), their fields (name/sign/bio/portrait?) and what `Import`/`FindTheSimsInstallation` actually read.
  Core models only name/portrait/bio fields. [inferred]
  **ANSWERED 2026-10-21 (re-mysims-contracts4):** DIFFERS — templates = exemplars (type 0x6A297266, ExemplarType 0x26) plus MySim\MySims.dat records (magic 0x4A32E4AA; u8-length name, u8 male, u8 sign, u8-length portrait); Import reads The Sims UserData[i]\Export *.pub/.ini families; registry SIMS_DATA then InstallPath; mysims.md §4.12
  **CLOSED 2026-10-21 (re-mysims-contracts4, round 2):** the three items left inferred are now byte-read: gender IsEqual flag 0 = ASCII case-insensitive via MakeLower + memcmp (0x0090EC57, call 0x0043CDC2); portrait = record bytes + `.bmp` with no directory, joined to GetMySimDirectory() only at display (PickHead 0x007757F0); after Import with count > 0 and the cache loaded, 0x0043BB90 clears the cache and the loaded flag is reset (0x0043D006). Template offsets corrected (+0x00 bitmap, +0x04 portrait, +0x18 name, +0x2C male, +0x2D sign). mysims.md §4.12
<!-- from: core-mysims-pool4 (2026-10-21) -->
- **MS11 - section 4.7.1:** dispatch location details the job-search core needs: (a) when is a location "valid"
  for `SetCurrentCommuteDestination` step 1 beyond "the default (occupant flag set, cells -1) is invalid"?
  [occupant flag set: valid iff an occupant is held; flag clear: valid iff both cells >= 0]; (b) the default bytes of
  the neighbour-city fields `+0x14`/`+0x18` [not written; modelled as absent]; (c) what cell `SetByOccupant`
  stores (the lot cell reported with `0xEBA06BF4`) [the search's end cell]; (d) is dispatch `+0x40` the trip state of
  section 4.6 [kept as its own field]; (e) the `notify` argument `ChangeJob(b)` passes [false: no `0xEBA06BF4`].
  **ANSWERED 2026-10-21 (re-mysims-pool-answers4):** DIFFERS — (a) IsValid 0x00434F90: flag +0x1C set → valid iff 0 ≤ cell < city cell count on both axes (occupant not consulted); flag clear → region GetCity(+0x14,+0x18) non-null and a non-empty GetConnectionsOfTypeToCity for one of Highway, GroundHighway, Avenue, Road, DirtRoad, Rail, LightRail, Monorail, Subway (no Ferry). (b) default +0x14/+0x18 = −1/−1 (ctor 0x00431320), +0x20/+0x24 = 0, world −16.0. (c) SetByOccupant 0x00431430 resets to default (byCommute 0) then cell = floor(occupant GetPosition x,z / 16). (d) +0x40 IS the trip state: setState 0x00431C10 sends 0x4BB2CFC0(old,new) that feeds §4.6. (e) ChangeJob passes notify = true (0x00425D99): 0xEBA06BF4 is sent. Rules in mysims.md §4.7.1 "Dispatch location contract".
<!-- from: core-mysims-pool4 (2026-10-21) -->
- **MS12 - section 4.12:** pool details left open: (a) the template fields when an exemplar property is absent
  (`0xEA296F8D`, `0xEA296F8F`, `0xEA296F91`, `0xEA296F90`) [0 / empty / false / 0]; (b) the ini reader's section and
  key matching (case) and its integer parse for `sims_family_nummembers` / `_guid` [exact bytes; atoi-like decimal];
  (c) the exact bytes of the picture folder ("install path, `Web Pages` and `FamilyGFX` with that value") and of the
  `family<k>_face.jpg` source [`<install>\Web Pages\FamilyGFX\<value>\family<k>_face.jpg`]; (d) `F.ini` directory
  [the `Export` directory of `F.pub`]; (e) `Import` record strings longer than 255 bytes [cut to 255];
  (f) `<path>\UserData` when the registry path already ends in `\` [plain concatenation, double separator kept];
  (g) the "male" text or a zodiac name failing to load [empty string].
  **ANSWERED 2026-10-21 (re-mysims-pool-answers4):** DIFFERS — (a) absent props write nothing and the template ctor 0x008B5E70 leaves bitmap/is-male/sign uninitialised: name and portrait empty, the other three indeterminate in the original (reimplement as 0/false/0). (b) cRZIniFile ReadEntry/ReadNumber: ASCII case-insensitive section/key, sscanf("%ld") decimal; nummembers result untested, unsigned k ≤ n loop. (c) picture folder = <ud>\Web Pages\<value>\FamilyGFX\ under the UserData dir (not install path); source cRZPath name family<k>_face ext .jpg. (d) F.ini = Export dir + name minus last 4 bytes + .ini (MATCHES). (e) length byte = len mod 256, that many bytes written (not cut to 255). (f) "%s%sUserData" with "\" always inserted: double separator kept (MATCHES). (g) cSLString miss → empty string (MATCHES). Rules in mysims.md §4.12.

<!-- from: core-mysims-contractfix4 (2026-10-21); answered by re-mysim-final-gaps4 -->
- ~~**MS13 - section 4.7.1:** location details left after MS11: (a) `0x00431370` `floor(world / 16)` for a NaN or a
  quotient outside `i32` (x87 integer-indefinite `0x80000000`, or something else?) [Rust saturating cast, NaN → 0];
  (b) does `SetCellPosition(x, z)` (neighbour branch of `validateCommuteLocation`) also write the world fields
  `+0x04`/`+0x08`, or do they keep the −16.0 of the step-1 reset? [cells only]; (c) the home start rect when the home
  holds an occupant whose lot is not found: the lot at the home cell, or straight to the single cell? [single cell].~~
  **ANSWERED 2026-10-22 (re-mysim-final-gaps4):** DIFFERS — §4.7.1: positionToStandardCityCell uses the 24-bit x87 magic-number bit algorithm (00431370–004313FD), not saturation or integer-indefinite; NaN 7FC00000 gives 880803840. SetCellPosition writes world as well as cells (004311B0–004311D7). An occupant whose lot lookup fails goes straight to its single cell (00431880–004318D7; c MATCHES). Vectors: §12 case 21.

- ~~**MS14 - section 4.12:** `Import` details left after MS12: (a) the value of the family member count `n` when
  `sims_family_nummembers` is missing or not a number (`ReadNumber` writes nothing and its result is not tested: is
  the local pre-initialised, and to what?) [0]; (b) the text of the member keys and of `family%d_face` for `k ≥ 2^31`
  in the unsigned loop (printed with `%d`, negative, or `%u`?) [`%d`]; (c) `F.ini` for a listed entry name shorter
  than 4 bytes (`Left(len − 4)` with a negative count) [empty stem]; (d) a missing `_gender` / `_zodiac` entry:
  skip the member, or an empty value? [empty value].~~
  **ANSWERED 2026-10-22 (re-mysim-final-gaps4):** DIFFERS — §4.12: n is zeroed per family at 0043C27E (16 bytes of pending pushes), passed to ReadNumber at 0043C761 and tested unsigned at 0043C84D (a MATCHES). Member/face formats use signed %d (0043C9BF,0043CA45,0043CB60,0043CBE4,0043CC8C; b MATCHES). Left's unsigned compare at 0090F46D–0090F478 keeps the whole short name. Missing gender or zodiac skips the member (0043CAA6–0043CB44,0043CBBE–0043CBC2); a successful empty read continues. Vectors: §12 case 22.

- ~~**MS15 - sections 4.6, 4.7, 7.2:** trip and commute details not published: (a) `CheckCommute` (`0x0042D9E0`):
  §4.7 says "the pathfinder's slot use is in §7.2", but §7.2 lists none. Which pathfinder slots and values does it
  call (flags, destination filter, destination flags, travel-type masks, density, max trip length, start/end rect
  source: the MySim's job/neighbour fields or the dispatch's commute destination), which `FindPath` result counts
  as success, and which getter is "totalTime" (slot)? Is the pathfinder created only when a home and a job exist,
  and does a missing home/job lot call `FailCommute`? [implemented: host callback `commute_time`, no core
  configuration]. (b) §4.6: is `trip_traffic_congestion` read with `PositionToTract`/`GetTractValue` at the current
  position ("All grid reads ...") or at the cell? [position]. (c) §4.6: does the `force` reset of the cell to
  (0, 0) also apply to `GetTravelTimeRatio(cellX, cellZ, …)`? [yes, same cell]. (d) §4.6: what is the class's
  "active" check of the gate? [MySim state != 0]. (e) §4.6 `destination_subject` "when travelling": the new trip
  state = 2, or the dispatch's current state = 2? [new trip state].~~
  **ANSWERED 2026-10-22 (re-mysim-final-gaps4):** DIFFERS — §§4.6/4.7/7.2: CheckCommute 0042D9E0–0042DB40 uses slots 5(1),9(3),18(1),19(home occupant bounds),20(job occupant bounds or MySim stored commute cell),23; success is exactly 1, then slot26 scaled-display getter and _ftol2 low32 store without clamping. No lot lookup or dispatch destination source. Missing references/null factory output return false without FailCommute. Other results run FailCommute fully before Release, including replacement-search owner callbacks (0042C8E0–0042C930). Trip congestion uses local world position (00427D1C–00427D4A), ratio local cells including forced zero (00427C60,00427D85–00427D96); active is state != 0 (00449550); subject uses new-state argument (00427980). b–e MATCHES, with local-copy/subject-retention rules added. Vectors: §12 cases 23–25; inherited factory defaults remain MS16.

<!-- from: re-mysim-final-gaps4 (2026-10-22), native CheckCommute request audit -->
- **MS16 - section 4.7 / 7.2:** CheckCommute leaves destination flags, travel-type masks, starting costs,
  maximum trip length and maximum work time untouched. What exact configuration does traffic
  `CreatePathFinder` slot 6 (`0x0070D860`) leave in the returned object (constructor plus Init/factory
  overrides)? [unproved; inherit the returned object, no invented numeric defaults]. Constructor bytes
  at `0x006D89C0` alone do not establish the factory result. The static export does not decode the factory,
  its decomp file lacks it, re_query's raw-disassembly fallback lacks capstone, and REA returns
  `target_unavailable: No app is open`. Resolve from factory/Init bytes before asserting numeric defaults.
  **ANSWERED 2026-10-22 (re-mysim-pathfinder4):** MATCHES — §§4.7.2/7.2: factory bytes 0070D860–0070D89F prove fresh constructor, AddRef, Init, borrowed traffic pointer, output publication and AL=true, with no request overrides or Init-result test. Init 006D78B0–006D7951 only initializes distance tables. Inherited strategy=3, destination byte=0, allowed/starting uint16 masks=2/2 (Car ID 1), costs=null (traffic-array fallback), max trip=float32 +0.0 and max work=uint32 0. Flags/density start 0/0 and CheckCommute changes them to 1/1. Flag 1 bypasses cost-budget rejection; work time 0 disables timed suspension. One owned reference; native null-allocation/null-out paths fault rather than normally returning false. Evidence obtained with installed MSVC dumpbin after export fallback and REA failed. Discriminating cases: §12 case 26. No remaining MS16 contract gap.
