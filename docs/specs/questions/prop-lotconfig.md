# prop-lotconfig: open questions
<!-- Questions about [prop-lotconfig.md](../prop-lotconfig.md). Append new entries at the END; when one is answered, move it to answered/prop-lotconfig.md. Rules: README.md. -->

<!-- from: the glue21 fix round (2026-10-20; the lane's GLUE21-3) -->
- **PL15:** (§ shadows, glue21's `FC4PropLotShadow.cpp`) what does a shadow compare when a replacement is on? Since
  glue21 a replacement (`enable = propmgr` / `lotcfg` / `tutorial`) does NOT turn its shadow off: the shadow reads
  whatever the city holds (FreeCity4's class when the replacement is in, the game's otherwise), writes that class's
  own record and runs a private core over it on a read-only host. Over a replacement that is a class-layer self-check
  (ours against our own private core), not an independent differential - "shadows exact" over a replacement is
  therefore weaker than over the original. [that reading is intended]

<!-- from: the glue21 fix round (2026-10-20; the lane's GLUE21-4) -->
- **PL16:** (§B7, the SDK header's `cISC4LotConfigurationManager`) the header declares slot 5 as
  `AddLotConfiguration(cISC4LotConfiguration*, bool)` and slot 6 as
  `AddLotConfiguration(cGZPersistResourceKey const&, void*)`, but B7's game order (and the `ret` column of
  `docs/abi/interfaces.tsv`) is the reverse: slot 5 takes ONLY the key (pop 4) and slot 6 takes the configuration
  and the flag (pop 8); the set queries return the count in EAX where the header says `bool`. glue21 corrected
  its plugin-wide declaration (`FC4TractInterfaces.h` `cFC4ISC4LotConfigurationManager`) against B7; no call site
  used the wrong shapes. Confirm the renames are the right fix for the whole plugin (an SDK erratum), rather than
  a per-lane override. [yes - B7 is the measured order]


<!-- from: the glue21 fix round (2026-10-20; the network-lot fault after the QI/creation-ref fixes) -->
- **PL17:** (city-detail.md §3.4 → network-manager.md §8.6) after the QI-AddRef and creation-reference fixes the
  city Init reaches PostCityInit and the three shadows go ON over the replacements, but the run then faults in the
  game's own `CreateLotObjects` (`SimCity 4.exe+0x60D4B5`, inside `0x0060D410`; `ACCESS_VIOLATION` reading
  `0x06E932F8`, `ecx = 0`, `eax = edx = 0x01BA4CBE`) on the network-lot redevelopment path: city detail Init §3.4 →
  `networkLotManager.RedevelopLot(x, z, lot)` (slot 12) → `CreateLotObjects(lot)` → the object array of the
  network lot's `cSC4NetworkLotConfiguration`. Whose object array is being read there (the network configuration's
  `objects` vector is the game's own), and what makes it unmapped at that point? The two FreeCity4 replacements
  that could be involved are `GetObjectIDArray` (slot 27, ours, called by CreateLotObjects for prop objects) and
  the prop manager's `AddStreetProp` (slot 32, ours, called by CreateProp).


<!-- from: the glue21fix2 round (2026-10-20; the configuration-lifetime fault after the PL17 fix) -->
- **PL18:** (§B5.1, §B5.2, §B5.4, prop-lotconfig.md §B7 slot 20; the SimNewDay growth-pass AV) with
  `enable = lotcfg` the city load completes and the run reaches turbo, but the growth developer's
  SimNewDay pass faults: AV reading 0 inside our `cFC4LotConfiguration::IsCompatibleWithPurposeType`
  (std::vector<bool> on a wrapper deleted out from under the catalogue map), deterministic at the
  first SimNewDay with configurations. Measured with per-op ref tracing (glue21fix2-iso-lotcfg4/5,
  isoref-1..4): each configuration's count over its life is creation +1 (the map's own reference,
  core add_configuration), +1 raw per `GetLotConfigurationMap` rebuild (our scratch map,
  InsertConfig), +1 per set-query insert (the §B5.2 reference, kept on duplicates per quirk 12);
  then the game's growth pass (SimCity 4.exe+0x309798 / ILT 0x00709798) RELEASES FOUR references
  per configuration and the wrapper deletes while the catalogue map still holds the pointer.
  Which references does the game's growth pass believe it owns (its own acquisitions at load -
  a bulk +1 wave of one reference per configuration that is not ours - and its per-query
  release convention), and how should a replacement's slot 20 scratch map / slot 9 set queries
  hand out references so the game's release pattern balances? The FreeCity4 readings
  (slot 20 = a rebuilt scratch map holding one reference per configuration; slot 9 = the §B5.2
  reference per insert, duplicates included) evidently do not match the game's expectations.


- **PL20:** (§B5.2, §B7 slots 9-12; docs/abi/allocators.md row 81 and *hash_set*) which hash function does
  the game's `hash_set<cISC4LotConfiguration*>` use to place a value in a bucket (the one the growth pass's
  iterator increment at `0x0047C1A0` recomputes when a chain ends, and insert `0x0048A170` uses)?
  allocators.md specifies only `hash_set<uint32_t>` (identity, v mod n). Measured (glue21fix2-pin-1, per-op
  trace, `build-runs/wave_walk.py` in the glue21 worktree): our slot 9 put 89 configurations into the
  growth pass's 193-bucket set at v mod 193; the walk released 8 of them once and then cycled over 6
  forever (05397290, 037CF900, 037CD8F0, 05397878, 037CDFB0, 05397A28 = buckets 10, 10, 101, 171, 92,
  24), which is PL19's "drain to zero". Every chain-end jump of that walk (26 of 26) matches an increment
  that recomputes the bucket as (v >> 2) mod n; identity matches 0. FreeCity4 now inserts with
  (v >> 2) mod n for slots 9-11 and walks slot 12 in bucket order (`PointerHashU32`, engine/plugin/
  FC4GameList.h). Please confirm the hasher (and whether resize rehashes with the same function), and
  add it to allocators.md.

<!-- from: glue-lotcfg-detail4 (2026-10-22) -->
- **PL22 - §B3.1 steps 1-2, in-game catalogue reload on a detail change:** the lotcfg host now reads `d` from the
  preferences (`videoPreferences.cityDetail`) at each `StaticInit` and each slot-5 load. A harness slot runs one city
  in one process at the slot's fixed preference (1), so only step 3 (first load at d = 1) can be observed in game:
  no published script action or harness key changes the preference, and no action opens a second city in the same
  process, so step 2 (unload at another level; the family set kept) is covered only by the core and C ABI tests
  (`t8_rebuild_on_detail_change`, `the_catalogue_and_its_rebuild_on_a_detail_change`). Is there a sanctioned way
  (a harness key for the slot's preference, or a script action that changes the preference and reopens or
  re-initialises the city) to exercise the reload in game? [none known; reload verified only offline] Also: is a
  preference byte outside 1..3 possible? [passed through unclamped, as the spec names only 1..3]

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

  Follow-up evidence (glue-dormant-shadows5, 2026-10-23): combined native-owner run
  `dormant5-all-2032-20261023-r1a`, DLL SHA256
  `e1944d66d824d1178f3f64b3bd7ca4c1f5cb5c60a4568ae612c3b3668b8d362d`,
  clean at 2032-01-01, ref_warnings=0, reproduces all 1544 padding-only profile
  differences (keysDiffering=0, validDiffering=0, profilesDiffering=0, padOnly=1544)
  and 1908/1909 configuration-array differences with 1908 foundations. Prop shadow
  lines are 219/220 clean, lotcfg 292/293, with zero incomplete or resynced lines.
  No defaults or comparison filters were changed; both keys remain ineligible for
  an exact shadows profile pending this contract. This adds evidence to PL23,
  rather than creating a duplicate question.


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
