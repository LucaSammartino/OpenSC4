# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

<!-- from: the mysims core audit (cores26, 2026-10-20): mysims.md §11.2's open points, first filed -->
- **MS2 (§11.2.2, §4.2):** UI service `0xA417445E` (instance `0x5A4`) slot `+0xA8` — which window or state blocks
  the MySim daily and monthly steps? [inferred: a modal window]
- **MS3 (§11.2.3, §2.2):** does MySim `Init` re-resolve the neighbour job city from the saved cell (the app's
  region lookup), and does it set `neighbor_job` true only when a city is found? [inferred: yes]
- **MS4 (§11.2.4, §4.9):** the building development simulator's slot 44 list used by `DistanceToClosestSpecial`
  (§7.2): landmarks and rewards only, or every special building? [inferred: landmarks and rewards]
- **MS5 (§11.2.5, §4.5):** the exact clamp and neighbourhood of `local_traffic_congestion` (the Mac build shows
  the rectangle grown by 1 cell on each side, clamped to `[0, CellCountX/Z − 1]`), and the neighbour-job constants
  of `job_condition` / `job_has_power` / `job_has_water` (0 / 1 / 1 in the core). [inferred]
- **MS6 (§11.2.6, §4.7):** does the traffic pathfinder `CheckCommute` and `FindNewJob` run have any side effect on
  traffic data (a recorded trip)? [inferred: no]
- **MS7 (§11.2.7, §4.1, §4.6):** the "announce" check of `AnnounceMoveIn` (the class slot that passes on the
  reload case), and the dispatch state numbering 4/5/6. [inferred: announce is skipped after a load]
