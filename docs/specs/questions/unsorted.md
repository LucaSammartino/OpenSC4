# Unsorted open notes

Notes that name questions not yet filed in a per-spec file.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31), "Not settled by this lane" -->
Filed by re-answers31 (2026-10-16) as not settled. Update 2026-10-20: TR24-TR27 have since been answered (see answered/terrain.md); the PO questions live on branch re-answers27; NR9-NR13, glue19 A2/A3 and cores21 Questions 1-12 still need to be re-filed in their spec's file by their lanes.
- NR9, NR10, NR11, NR13 (cores22: the tile grid width field `+0xC4` and `GetTileInfo`, the mark/remove cell
  set, stale `AddTypes` links, bridge/rule-solver coupling). Not answered here; each needs its own pass over
  `network-rules.md` §10 and `network-bridges.md` §7-§8.
- TR24 - TR27 (glue18: the §15.1 terrain slots, the surface-water `IsNavigableAtPosition` argument dwords,
  the water's message target and animation tick). Not answered here.
- PO2, PO3, PO5, PO6, PO1b (cores21 / `prop-occupant.md` §10: the sixth sub-object at `+0x24`, the
  occupant-to-placeable-to-3D seam, terrain re-solving of a prop's box y, the SDK names of slots 5 and 6, the
  stride-`0x88` allocator group). Note `prop-occupant.md` exists only on branch `re-answers27` and is not on
  `main`.
- glue19 A2, A3 and cores21 "Questions 1-12" were never written into a tracked file; the lane reports
  were trimmed to their merge-train rows. They have to be re-filed by their lanes before they can be answered.
