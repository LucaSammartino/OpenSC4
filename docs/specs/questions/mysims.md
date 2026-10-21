# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

<!-- from: the mysims core audit (cores26, 2026-10-20): mysims.md §11.2's open points, first filed -->
- **MS5 (§11.2.5, §4.5):** the exact clamp and neighbourhood of `local_traffic_congestion` (the Mac build shows
  the rectangle grown by 1 cell on each side, clamped to `[0, CellCountX/Z − 1]`), and the neighbour-job constants
  of `job_condition` / `job_has_power` / `job_has_water` (0 / 1 / 1 in the core). [inferred]
- **MS6 (§11.2.6, §4.7):** does the traffic pathfinder `CheckCommute` and `FindNewJob` run have any side effect on
  traffic data (a recorded trip)? [inferred: no]
- **MS7 (§11.2.7, §4.1, §4.6):** the "announce" check of `AnnounceMoveIn` (the class slot that passes on the
  reload case), and the dispatch state numbering 4/5/6. [inferred: announce is skipped after a load]
