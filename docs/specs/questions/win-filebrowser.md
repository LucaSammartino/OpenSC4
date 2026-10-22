# win-filebrowser: open questions
<!-- Questions about [win-filebrowser.md](../win-filebrowser.md). Append new entries at the END; when one is answered, move it to answered/win-filebrowser.md. Rules: README.md. -->

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ1 - sections 8.3 / A:** What are the exact procedural icon, arrow and button-buffer pixel recipes, selector mapping, buffer initialization format, lock-failure results and Boolean success channels? Large bodies `0099EF11` (3423 B), `009A1163` (1938 B), arrow `009A0FEE` (373 B), loader `009A0EAD` (321 B). Cached pseudocode misrecovers stack arguments; null-icon-buffer success and button initialization-failure release/null are byte-proved, but the rest needs byte reconstruction. [unknown; do not claim pixel conformance or infer success from a non-null buffer]

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ2 - sections 8.2 / 8.5 / A:** What are the full control creation/configuration sequence, EnableChildControl effects, geometry/layout, size constraints, popup visibility/focus and paint-background placement? `009A1F77` (3149 B), `0099FDE6` (615 B), `009A0ACC` (687 B), `009A0A17` (173 B), plus primary table `00ADD190`. Partial control IDs/factory ownership are specified, not complete layout or event dispatch. [unknown; do not infer from core or extend the separate Grid spec]

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ4 - sections 6 / A:** What are the exact meanings, argument widths, ownership and return channels of the remaining primary slots, including simple property methods, filter mutation, and programmatic pending selection? `00ADD190` has 107 slots; current ABI export omits this primary interface. Slot 90 (`009A00FC`) is byte-proved in section 6.1: it fills a five-word selection descriptor with type, ID and three borrowed string-object pointers. Reconcile `009A323A`/`009A2EC7` before specifying deferred selection. [other slots unknown; never treat a notification descriptor as a string or its borrowed fields as owned copies]

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ5 - sections 8.1 / A:** What is the complete file comparator and directory/drive ordering, wildcard option semantics, column-width arithmetic and pending-selection restoration? Audit `009A3FA7` (547 B), `009A4EBA` (2241 B), `009A4B2E` (908 B), `009A473F` (1007 B), and FitStringIntoWidth `009A3B67` (346 B). Accept-any-pattern and empty-filter acceptance are specified; full sort/selection behavior is not. [unknown; do not invent order, fallback row or pixel-width rounding]

<!-- from: re-filebrowser4 (2026-10-22) -->
- **FBQ6 - section A:** Which preserved reachability/neighbour/compiler helpers truly belong to the browser rather than shared containers or export boundary artifacts? The preserved set is 100 functions / 25579 B; its union with 106 distinct primary-table targets is 174 functions / 28907 B. In particular, `005C3A63` is a separately referenced comparator target with a suspicious decoded entry and is not primary slot 107. [keep attribution inventory separate from fully specified owner coverage; no complete-owner credit]
