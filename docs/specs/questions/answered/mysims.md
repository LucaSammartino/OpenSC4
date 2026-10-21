# mysims: answered questions (archive)
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
