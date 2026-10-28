# Neighbors questions


  **Follow-up, region-exact33 round 3 (2026-10-28):** Actual native pointer order in the plugin does not
  solve this gap. An opt-in experiment sorted the game's real regional-city pointers both before and after
  `ReloadCity`, retaining §14.1's reverse clone walk. Qcheck-16 still had only the regional-city record
  different: first byte 11881, 140 differing bytes. The seven complete deals remain an identical multiset,
  with the entire prefix and suffix exact. Actual groups became `(1,6), (0,5), (0,7)`; the reference remains
  `(0,7), (0,5), (1,6)`. Reloads ran `(0,7), (0,4), (1,6)` and their new pointers increased in that order;
  `(0,5)` is a deal endpoint inside the `(0,4)` city's footprint. Native allocation history after reload
  therefore still affects the clone order, even with original regional-city objects. The experiment is
  reverted; §9.1's specified position-order choice is retained. Please specify the fixed-schedule walk
  rather than requiring matching allocations. Evidence:
  `C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-034447-2370da38/result`.
  Final restored-source qcheck-19 returns to 1/2 exact: Indereast first byte 11881, 139 differing bytes,
  with the same actual record SHA256 as before the experiment. Evidence:
  `C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-040727-e0a67421/result`.
