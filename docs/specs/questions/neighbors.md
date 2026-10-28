# Neighbors questions

- **NB-EXACT33-1 — §9.1, §14.1 steps 5–7 / simulator §5.8:** What adjacent-city order must the replacement use under the fixed schedule? §9.1 explicitly chooses region-position order for FreeCity4 because the native address order cannot be reproduced. The fixed-schedule address-order list does not yet name this walk. That choice prevents exact saves in `maxisland-indereast`: after correcting PC24 commodity arithmetic, qcheck has only one differing record, `CA027EDB/CA027EE1/0`, first byte 11881, 139 differing bytes. Its seven complete version-5 local-deal records are byte-identical as a multiset; only their order differs. The reference groups other-side cities as `(0,7), (0,5), (1,6)`; ours groups them as `(1,6), (0,7), (0,5)`, the reverse of the specified position sort used by Init step 7. Please provide the deterministic ordering contract for this walk and reconcile the §9.1 choice with fixed-schedule exactness. Evidence: `C:/Users/tlori/FreeCity4/build-runs/golden-runs/compare-20261028-021402-82bbf8a5/result`; original-only qcheck was exact on the same scenario. `paradis-ragako` is now exact with neighbors enabled.

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
