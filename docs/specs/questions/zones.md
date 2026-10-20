# zones: open questions
<!-- Questions about [zones.md](../zones.md). Append new entries at the END; when one is answered, move it to answered/zones.md. Rules: README.md. -->

<!-- from: From the return-channel audit (2026-10-16) -->
- **ZG41:** lot manager slot 24 DeleteLot is "EAX undefined; error: no bool result" in zones.md §8.6 and §11.3, but
  lot-developer.md (cross-reference table near line 1659) and tract-developer.md's interface table (line 1164) list it
  as AL. Those two tables should be corrected. [our hosts ignore it and return true (F3)]

<!-- from: From the return-channel audit (2026-10-16) -->
- **ZG42:** (tract-developer.md, the lane's "Q8") the tract-developer.md interface table gives lot configuration manager
  slots 18/19 (Register/UnregisterLotConfigurationFilter) as AL, but prop-lotconfig.md §B7 gives "—" (no defined
  return). Which is right? [no defined return; the result is not tested (F6)]
  emergency's own `u16` first and misaligns, failing at the occupant base version. Which layout is correct?~~ **ANSWERED 2026-10-16 (re-answers20):** The §6.4 layout is correct and **quirk 3 was wrong** (automata.md §12 quirk 3, §6.4). The emergency Write (`0x0052FA60`) writes `u16` 2, the full vehicle record, then its own fields. Reloaded under the vehicle class ID, the readers shift by one `u16`: vehicle (2 ok), traffic (2 ok), automaton base (2, ≤ 8 ok). The occupant base then reads 8 where it needs 4 (`0x005E8D75`) and fails, so that object's Read fails; it does not stop partway with success. The cores14a description is right.
