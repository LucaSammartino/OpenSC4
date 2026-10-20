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
