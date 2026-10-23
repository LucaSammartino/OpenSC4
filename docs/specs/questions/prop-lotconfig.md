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

- **PL21:** (§A7.2 slot 23 `GetLotProps`, §A8 persistence) the propmgr shadow (originals in, FreeCity4's
  core loaded from the original's own record each SimNewMonth) differs intermittently in one GetLotProps
  sample per run, never the same lot: glue21-r4-3 (turbo) 2028-10-01 lot (10,46) original 3 props, core 2,
  first 0C3079EC/0C3075E0; glue21-r4-5 (normal speed) 2027-11-01 lot (9,51) original 1, core 0; glue21-r4-4
  (turbo, same build) 220/220 exact. Every other query and the record compare exact. Can the original's
  per-lot prop list hold a prop that its saved record (§A8) does not carry at the moment of a SimNewMonth
  Write (for example a prop added to a lot but not yet in the city's prop table, or a timed prop that the
  record skips)? If so, which props, so the shadow can exclude them.

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
