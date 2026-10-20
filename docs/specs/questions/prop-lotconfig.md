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

