# prop-lotconfig: answered questions (archive)

<!-- from: glue-propmgr-race13 (2026-10-24) -->
- **PL24 - prop-lotconfig.md A7.2 slot 24 upper-bound comparison:** does native `GetLotProps` retain positions equal to either upper world bound, contradicting the specified half-open rectangle? Please confirm both axes and the exact comparison/NaN behavior before changing the core. [Core retains the literal specified `wx1 <= px < wx2`, `wz1 <= pz < wz2`; no tolerance or skipped comparisons.]
  Unpatched final-artifact follow-up (2026-10-24, source `424731910`, DLL `b65c4f6d`): the full gate passed, then both final-artifact runs reached 2032 clean/ref_warnings=0 with zero resyncs but **219/220**, including normal speed. Turbo `propmgr-race13-r3-final-a` (run 737), 2026-11-01 lot(4,59): native/core lists 3/2; native-only 1F8B1174 observed at (77.1937561,266.25351,976), observed lot world bounds (64,944,80,976). Normal `propmgr-race13-r3-final-b-normal` (run 738), 2027-07-01 lot(22,51): native/core 2/0; native-only 1F63CE90 and 1F63C95C both observed at Z=848, observed bounds (352,816,400,848). All query vtable patches and capture/replay paths are absent. Thus removing slot-40 capture does not eliminate the mismatch: the general boundary contract is still required. These are ordinary post-query observations, not an atomic native dependency snapshot, so they do not justify guessing a replacement comparison or NaN rule. Logs: final-a lines 3339-3347; final-b-normal lines 3728-3733. No outputs were imported, comparisons skipped, or tolerance added. Earlier runs 735/736 used the same engine source but DLL 39ab9efd and happened to be exact; do not treat them as proof of a permanent mismatch fix or grant final-artifact verified credit.
  Safety correction (2026-10-24, source `424731910`): the earlier capture probe patched shared occupant/city/occupant-manager vtables and crashed train207 inside a native lot-occupant filter. Its instrumented inputs cannot establish an unmodified-native ordering or boundary rule. All three patches and all dependency replay have now been removed. Ordinary live native/core comparisons reached 2032 clean at **220/220** in both turbo `propmgr-race13-r3-readonly-a` (run 735) and normal `propmgr-race13-r3-readonly-b-normal` (run 736), zero resyncs and reference warnings. These two exact runs validate patch removal but do not prove the cause of every previous intermittent mismatch; the general PL24 contract remains open. No core rule changed.
  In `propmgr-race13-house2032-a` (Konradshohe, turbo to 2032, 219/220 exact, clean/ref_warnings=0), the 2026-12-01 query at (7,45) has captured slot-40 rectangle (7,45,8,45). Its only native slot-8 observation is occupant 1F8BDAB4 at (144,267.618225,726.247192), exactly on upper X=(8+1)*16=144. Native returns [1F8BDAB4]; Rust returns []. Both consume the same ordered slot-40 list and the exact native slot-8 return values, with inputErrors=0. This rules out a position-capture timing difference for this failure; native did not skip that position read. Prior coordinator train200/train195/train190 failures likewise retained props at apparent upper Z boundaries 624/976/592 (post-query observations only). Evidence: build-runs/game-results/propmgr-race13-house2032-a.log lines 3386-3390 and build-runs/propmgr-prior-failures.md. Please establish the rule in the spec; no guessed native signature or rule has been implemented.
  Normal-speed confirmation: `propmgr-race13-house2032-b-normal`, 2026-03-01 at (2,58), captured rectangle (2,58,3,58), native slot-8 occupant 1F7E84BC at (53.6957321,281.939667,944). Upper Z=(58+1)*16=944; native returns this sole entry while Rust rejects it, inputErrors=0. Clean to 2032-01-01, ref_warnings=0. Thus the upper-Z discrepancy also survives identical observed inputs at normal speed (log lines 2957-2961).
  Follow-up (2026-10-24, source `c5d532838`): the earlier probe still independently called city slot 96 for Rust's bounds; those world-bound outputs were not captured from the native pass. The probe now captures and replays both actual native slot-96 calls, validating their count, order and cell arguments as well as all slot-40/slot-8 inputs. `propmgr-race13-house2032-c-bounds` (turbo, run 733) and `propmgr-race13-house2032-d-bounds-normal` (normal, run 734) both reached 2032 clean/ref_warnings=0 at **220/220**, with zero input errors and zero resyncs. The core still applies the literal half-open contract; no boundary tolerance or output import was added. This closes the remaining bounds-input capture gap, but does not establish an inclusive native rule or resolve the general comparison/NaN question. The earlier arithmetic upper bounds should not be treated as captured native bounds.

<!-- from: propmgr-query27 (2026-10-26; diagnostic source c599ed479) -->
- **PL24 additional ordinary-query evidence:** please specify the upper-bound contract for the following untyped, untimed lot props before an engine change. The core still follows the strict upper bounds in A7.2.
  `propmgr-query27-diag-a2`, 2031-12-01, lot(31,47), native/core 4/3: native-only 0C447B14 at (516.009033,277.440948,768); observed lot bounds (496,736,544,768). Core host position is identical; Z bits are 44400000 (exactly 768). Main build-runs/game-results/propmgr-query27-diag-a2.log:40212-40225.
  Artifact correction on resume (2026-10-26): the previously cited `propmgr-query27-diag-a3` X=144 sample cannot be substantiated from its currently archived log, which reports 220/220 with no `propmgr query difference` entry. Exclude that sample pending recovery of its original artifact; a2 above remains verified.
  The verified a2 occupant: type A823821E, GetPropUsage=2, GetPropType=0, flags=7, state=0, timer bits=0, chance=100, time range=(0,0), lot tag=0. All eight night/trigger/time/date/power/chance/duration/start properties from prop-developer.md 4.6 are absent. No successful single-occupant insert/remove was observed in the same framework tick; this tracker follows occmgr single-call hooks and does not claim coverage of bulk removal calls.
  The core sampled the a2 occupant from the live spatial query before rejecting its position. Bounds and the diagnostic position are ordinary post-query reads; native dependency inputs were not intercepted. No native vtable was patched, no dependency replay or output import was used, and no timing/state exclusion or tolerance was added. Please establish both axes and NaN behavior, including any special contract for GetPropType=0; these observations alone do not authorize changing the specified comparison.

  **ANSWERED 2026-10-26 (re-pl28):** DIFFERS — native slot 24 retains both upper-bound equalities:
  `wx1 ≤ px ≤ wx2` and `wz1 ≤ pz ≤ wz2`. Its ordered tests are lower X, lower Z, upper X, upper Z;
  any unordered comparison rejects (NaN X/Z or bound), Y is ignored, and no tolerance is applied.
  The same rule applies to `GetPropType=0`; this pass does not inspect that ID. It uses each
  candidate's slot-8 position and the actual city slot-96 bounds. See prop-lotconfig.md §A7.2 and
  §T.4: binary32 comparisons at `0x004A3316`–`0x004A3350`, including upper-X equality at
  `0x004A3334`–`0x004A3341` and upper-Z equality at `0x004A3343`–`0x004A3350`. Fixtures cover
  exact bounds, one binary32 step inside/outside, NaNs and caller-supplied entries. This is static
  instruction evidence, not new game verification. The archived a3 correction and the limits of
  the ordinary post-query observations above remain applicable.

- **PL21:** (§A7.2 slot 23 `GetLotProps`, §A8 persistence) the propmgr shadow (originals in, FreeCity4's
  core loaded from the original's own record each SimNewMonth) differs intermittently in one GetLotProps
  sample per run, never the same lot: glue21-r4-3 (turbo) 2028-10-01 lot (10,46) original 3 props, core 2,
  first 0C3079EC/0C3075E0; glue21-r4-5 (normal speed) 2027-11-01 lot (9,51) original 1, core 0; glue21-r4-4
  (turbo, same build) 220/220 exact. Every other query and the record compare exact. Can the original's
  per-lot prop list hold a prop that its saved record (§A8) does not carry at the moment of a SimNewMonth
  Write (for example a prop added to a lot but not yet in the city's prop table, or a timed prop that the
  record skips)? If so, which props, so the shadow can exclude them.

  **ANSWERED 2026-10-24 (re-propmgr-pl21):** DIFFERS — there is no separate per-lot collection or timed-prop
  save exclusion. With an empty caller list and stable state, newly returned props come from the occupant
  grid, whose writer references every node (`0x004A32AD`, `0x0049EE10`–`0x0049EE27`); the 18-byte manager
  record itself holds only manager references (`0x004A2430`). Nonempty caller lists retain interior entries
  even when unregistered, and a save/query pair has no shared atomic snapshot. Documented in §A7.2, §A8,
  quirk 20 and §T vector 17 (PL21-G/U/C/R/W/S). No props may be excluded on the proposed timing/persistence
  rationale; preserve the intermittent mismatch as unexplained game evidence, not an exact result.

<!-- from: verify-propcfg4 native comparison (2026-10-22) -->
- **PL23 - A3.2 profile padding and lot-configuration §3.7 foundation IDs, raw comparison evidence:**
  `vpc4-r2-native-20261022a` (immutable DLL SHA256 prefix `04b93bd7`, native city owners, clean2032/ref0)
  reports 1544/1544 prop profiles differing only in the three unspecified bytes after wealth, and 1908/1909
  complete configuration arrays differing (1908 appended foundations; the six printed first differences are
  the final record). The published contracts explicitly leave these bytes/IDs unwritten and choose zero in
  FreeCity4. What deterministic raw-equivalence contract, if any, can apply to these fields without copying
  native bytes or filtering mismatches? [none inferred: retain every raw difference and no whole-owner exact
  credit; do not change algorithms or defaults to match unspecified native memory; other array differences
  have not been ruled out by a field-by-field diagnostic]

  **ANSWERED 2026-10-23 (re-proplot-pl23):** MATCHES — no deterministic exemplar-derived raw-equivalence
  contract exists for the ordinary profile's +0x15..+0x17 bytes or the appended foundation's +0x24 ID. The
  original preserves prior stack contents and copies them into its output (profile byte write/read
  `0x004A3E52`/`0x004A3E5A`, seven-dword copy `0x004A3FDE`–`0x004A3FE7`; foundation construction
  `0x0047A59E`–`0x0047A609`, ID copy `0x0047892B`–`0x00478931`). Keep FreeCity4's explicit zeros, every
  raw difference, and the restriction on whole-owner exact credit. Defined-field comparisons must report
  their narrower scope and inspect every field; six final-record first differences do not exclude other
  array differences. Rules: prop-lotconfig §A3.2, §B3.3 and test 16; lot-configuration §3.7. This closes the
  behaviour question without claiming that the supplied raw comparisons now pass.
<!-- Archive for [prop-lotconfig.md](../../prop-lotconfig.md). Lanes do not read this file. Open questions are in ../prop-lotconfig.md. -->

## From the prop manager, lot configuration manager and tutorial cores (filed by the coordinator from the `cores15` lane's report, 2026-10-16; merged `03e2dcf`)

cores15 implemented prop-lotconfig.md (Parts A and B), the object reader of lot-configuration.md §3.5 and tutorial.md
as Rust cores (`engine/core/src/{prop_manager,lot_config_manager,tutorial}/`), C ABIs (`fc4_prop_manager`,
`fc4_lot_config`, `fc4_tutorial`, ABI 68) and conformance vectors (`plt_*`, `tu8_*`, `tu00_*`). There is no plugin
glue yet, so none of these readings has been checked in game. The current choice is in brackets. The lane numbered
these PL4-PL10 and TU3-TU4; they are filed as PL8-PL14 (plumbing's first-generation questions used PL4 and PL7) and
TU4-TU5 (tutorial.md already has TU3).

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL8:** (prop-lotconfig §A3.2) in what order does the profile reader `0x004A3DB0` apply its properties (class,
  size, satisfaction, wealth, crime)? OccupantSize values past the third overwrite the dwords from +0x10 on; values past
  the profile's end are dropped (the original writes past its local copy). [that table order, each over the defaults]~~ **ANSWERED 2026-10-16 (re-answers25):** **DIFFERS** in the overflow (prop-lotconfig §A3.2). The order is size first (`0x004A3DC4`–`0x004A3E11`: count and raw data pointer, copied with no bound; absent → three zeros), then class (default `0x2977AA47`, `0x004A3E20`), satisfaction (`0x004A3E38`), wealth (low byte only, `0x004A3E52`) and crime (`0x004A3E6D`), each default written just before its read. So extra size values are overwritten again: value 4 and value 6 vanish, value 5 survives in the upper three bytes of +0x14, and a seventh value overwrites the reader's return address (the local profile is the top 0x1C bytes of the frame): a crash, not a drop. A size with fewer than three values (or a scalar, count 0) leaves the missing size dwords unwritten (stack garbage). The bracket's table order and "each over the defaults" MATCH for the four scalar properties; *[choice]* FreeCity4 copies at most three size values, zero-fills the rest and keeps +0x14's upper bytes 0 (unreachable with shipped data). Test 13.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL9:** (§A6.1 step 5) what bounding box does an OccupantSize with one or two values give? [the missing values are
  0 (the building loader's bracket, DG32); an absent or empty size gives (1,1,1)]~~ **ANSWERED 2026-10-16 (re-answers25):** **DIFFERS** (unreachable with shipped data; the bracket is accepted as a *[choice]*, as for DG32). `0x004A2A70` passes the OccupantSize value's raw data pointer (variant slot 111, `0x004A2B72`) to `SetBoundingBox` as the far corner (`0x004A2B78`–`0x004A2BBE`), with no count or type check: one or two Float32 values read past the heap block (garbage, not 0), an empty array or a scalar gives a null or bogus address (garbage or a fault), integer arrays give their bits as floats. Absent → (0,0,0)–(1,1,1) MATCHES. prop-lotconfig §A6.1 step 5.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL10:** (§A7.1, slot 36 AddBuildingProp) when the building has no property holder, does the original skip the
  property step and still set usage 3 and return true? [yes *[inferred]*]~~ **ANSWERED 2026-10-16 (re-answers25):** **DIFFERS** in the usage. With no property holder the slot returns true at once (`0x004A3BEB` sets the result, `0x004A3BF6` jumps to the exit), skipping both the property step and the final usage write. The prop's usage is 3 only when this call inserted it (step 2, `0x004A3BDF`); an already-inserted prop keeps its usage. prop-lotconfig §A7.1.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL11:** (§B4.3, `0x0048AD40`) how does a building type whose exemplar does not load affect the building
  information? lot-configuration.md §4 does not cover it. [monopoly property absent (sets +0x8C), no groups
  *[inferred]*]~~ **ANSWERED 2026-10-16 (re-answers25):** MATCHES, with one addition (prop-lotconfig §B4.3, lot-configuration §4). The group collector `0x005E9500` loads the exemplar itself and returns nothing on failure (no groups); the monopoly flag is cleared before the load (`0x0048AE95`) and read only on success (`0x0048AEAA`), so a failed load sets +0x8C. The capacity comes from building development slot 39 (`0x0048AEE8`), not the exemplar, so the type **still counts** towards +0x3E/+0x40.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL12:** (§B5.7, CreateGenericLotConfiguration) what are the lot size and the object placement? [ceil(w/16) x
  ceil(d/16) cells, at least 1, for the slot 24 footprint, the building object centred *[choice]*]~~ **ANSWERED 2026-10-16 (re-answers25):** **DIFFERS** in the size. Each side is `1 − trunc(w × −0.0625)` (`0x0048BD66`–`0x0048BD9B`, `_ftol`), which is `1 + floor(w/16)`: one cell more than `ceil(w/16)` when w is a multiple of 16 (w = 16 → 2, 32 → 3; 4 → 1). The building object is centred on the lot (MATCHES): offset (8·sizeX, 0, 8·sizeZ), footprint ± (w/2, d/2), ID list [type]. Also: the ID is a fresh GUID (`cIGZCOM::CreateGuid`), maximum slope before foundation 5.0, and the object's lot tag (+0x24) is never written (stack garbage; *[choice]* 0). prop-lotconfig §B5.7, test 14.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL13:** (§B5.7, FixLotConfiguration) which validity test applies per record type, and how is the footprint
  recomputed? [building: BDS slot 16; prop: prop manager slot 11; flora: slot 11 or a non-empty family. The footprint
  is rebuilt from slots 24-26 (w and d swapped for orientations 1 and 3) around the old footprint's centre; one moved
  outside the lot by more than 0.049 m is shifted back in, a centre outside it is clamped *[choice]*. The result is
  the first problem's code]~~ **ANSWERED 2026-10-16 (re-answers25):** Validity tests MATCH, through the manager's held components (+0x14, +0x18, +0x1C): building development slot 16, prop manager slot 11, flora simulator slot 12 `IsFloraFamilyTypeValid` (a family or a registered type; equivalent to the bracket). Footprint **DIFFERS** in three ways: (1) a changed footprint is rebuilt around the record's **offset** (+0x04, +0x0C), not the old footprint's centre (`0x0048B667`–`0x0048B6A6`); (2) "outside the lot" (`minX/minZ < 0.049`, or `max > f32(lot − 0.05) + 0.001`) only sets code 4, **nothing is shifted** (`0x0048B6B3`–`0x0048B71F`); (3) an offset outside its footprint is **moved to the footprint's centre**, not clamped (`0x0048B727`–`0x0048B781`). The rebuild compares `|old − new| > 0` per axis after the orientation-1/3 swap. Only types 0, 1, 4 are examined. Result = the first problem's code (MATCHES). prop-lotconfig §B5.7, test 15.

<!-- from: From the prop manager, lot configuration manager and tutorial cores (2026-10-16) -->
- ~~**PL14:** (§B5.7) what counts as a valid retaining wall ID for the slot 28 list in FixLotConfiguration, and is only
  the first invalid ID removed (code 6)? [a host callback decides; the first invalid ID is removed]~~ **ANSWERED 2026-10-16 (re-answers25):** MATCHES on "first invalid only"; the rule is no host choice: an ID is valid when building development slot 28 `GetLotRetainingWallKeyFromType` (the held simulator, `0x0048B800`) returns true, i.e. it is the instance of an `ExemplarType 0x15` exemplar in the simulator's wall catalogue (`0x00698FC0`, lookup `0x00698250`). The first invalid ID is erased and the slot returns at once (`0x0048B822`–`0x0048B858`): the earlier code, else 6. prop-lotconfig §B5.7.

## From the parked glue21 lotcfg replacement (copied verbatim by re-pl19-run3, 2026-10-21)

- **PL19:** (§B5.2, §B7 slot 9; the SimNewDay growth-pass release wave) with the PL18 double-add
  (2 references per matched configuration per slot-9 query) the day-29 deletes are gone and the
  deletes move to day 30 and shrink to the configurations with no game-lot reference - but they do
  not stop: the game's release wave (SimCity 4.exe `0x00709798`) drains a touched configuration to
  ZERO whatever its reference count. Measured with the per-op trace restored
  (glue21fix2-ref2-2: no cushion; -ref2-3: a 40-reference diagnostic cushion per matched config):
  per configuration the load is create +1, core `add_configuration` +1, creation release -1
  (count 1), the game's own bulk wave +1 (exe `0x0069C385`), the game's per-candidate AddRefs
  (exe `0x006BF53D`, 1..66 per configuration over 736 of 1909), the slot-9 query adds (+2 per
  matched configuration: the core's §B5.2 add plus the plugin loop's), then the wave releases
  -1 per configuration per pass until the wrapper is DELETED: in ref2-3 five configurations of
  count 44 were released 43 logged times plus the deleting 44th; two others of the same count 44
  were released exactly once and stopped; in ref2-2 (no cushion) a count-4 configuration was
  released 4 times (deleted) and a count-5 one 4 times (survived at 1). The release count per
  configuration therefore follows the game's own candidate/pass structure, not the references it
  acquired (a configuration the game never AddRef'd was released 44 times), and no reference
  hand-out can balance it. Which container does `0x00709798` iterate (the growth developer's
  per-candidate list? the configuration set's entries per Grow pass), when does it stop, and what
  does the ORIGINAL do so its configurations are never drained to 0 (its catalogue reference is
  evidently never released)? Until this is answered from the exe, no FreeCity4 reading of slot 9 /
  slot 20 can make the lotcfg replacement game-clean; the double-add is kept (it strictly improves
  the ledger) and the fault stands between the run and a clean verdict.

  **ANSWERED 2026-10-21 (re-pl19-run3):** DIFFERS — `0x00709798` is the return address of the Release at `0x00709795` in `cSC4TractDeveloper::ListPossibleLotConfigurations` (`0x00709650`, once per Grow at `0x0070D4EA`); it walks the fresh local hash set that slot 9 filled, releases each element once, stops at the end of the set. The original balances it inside slot 9: one AddRef per returned element on EVERY call, cache hit included (`0x0048B29F`-`0x0048B2BC`); the filter cache holds no references. Net per configuration per query is 0; no extra add elsewhere (the PL18 double-add is wrong). A per-query drain means the add is skipped on some calls (e.g. cache hits) or lands on another object than the one put in the set. Rule: prop-lotconfig.md §B5.2 "Slot 9 reference rule", §B5.4, §B7 slot 9.
