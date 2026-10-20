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
