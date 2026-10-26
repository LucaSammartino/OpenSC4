# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: core-budgetwindow-ownership4 (2026-10-22) -->
<!-- Partial BW17 evidence: re-budgetwindow-bodies4, 2026-10-22. -->
BW17 is partially resolved in [budget-window-bodies.md](../budget-window-bodies.md): exact exported owner/slot
inventory, available-ordinance classification/map construction, six neighbour scans and counts, combo edits,
CalculateValues raw-input arithmetic, individual list/restore acquisition, and the bounded group-A refresh
slice. It remains open for the three bounded audit groups in that spec's section 7; no full-owner completion
or same-owner callback boundary is implied.

**Remaining paths for BW17 after this partial proof:** `0077ADA0` ordinance refresh/scale/B formatting;
  `0077DAB0` neighbour options and `00781C90` neighbour refresh/row metadata/reference balance;
  `00785740` service-query float/amount derivation and remaining control reuse, plus the malformed
  `00786F62` initializer continuation containing rebuild call `007875C5`;
  `007898A0` group B/slider rebuild, A item rendering and final stale-row cleanup;
  `007795A0` property-reference replacement, `0077BEC0` empty-window and `0077A480`/`0077A6F0`/`0077A960`
  setup/failure ownership, widget/popup factory failures. (The panel's loan, tax, formatting and balance-sheet
  rendering bodies left this list: budget-window-bodies.md §4.2–§4.3, §8–§10, re-budgetwin 2026-10-22; their
  summary parts were BW18, answered in §11 by re-budgetui7 2026-10-23.)
  See companion sections 1 and 7 for the full inventory; these remain questions, not accepted host inputs.

The original request follows.
- **BW17 - sections 1, 4, 4.1, 5.2, 5.6 and 8 whole-owner inventory / omitted bodies:** please publish the full method/slot inventory for `cSC4SubBudgetWindow` and the owned budget-panel paths, and the non-layout algorithms still omitted: ordinance enumeration/filtering, A/B counts and row-map construction in `initOrdinanceBudget`; proposed-deal enumeration, section/count rules and `+0x50` selection edits in `initNeighborBudget`/combo handling; occupant enumeration and the exact creation/ownership of `+0x44` individual restore records; all four per-mode refreshes (including group row reuse on scroll), empty/popup window creation and failure reference rules, and the panel loan-display/balance-sheet bodies. Which public external interface calls and arguments supply their raw inputs? [unimplemented: existing same-class populate/refresh host callbacks remain explicit gaps, not core coverage; no native own-class result may count as an external input. BW14/BW15 ownership and BW16 visibility are implemented from their settled text; the documented BW13 null-content divergence is retained. No whole-owner completion claimed.]
<!-- from: core-budget-window5 (2026-10-23) -->
- **BW19 - budget-window-bodies.md §3.2 and §6 invalid raw inputs:** what safe replacement behavior is required when combo-edit signed division has a zero divisor or INT_MIN/-1, or UpdateValues receives an empty budget-group vector? The literal native operations would fault. Core returns InvalidDivision for the affected row (continues matching rows and records the single pending neighbor refresh), or None for the empty group vector while clearing its busy guard. These are explicit safe divergences, not successful empty rows. [open]

<!-- from: core-budget-window5 (2026-10-23) -->
- **BW20 - §4 group initialization / budget-window-bodies.md §6 raw department list references:** §6 explicitly releases each department returned by budget slot 31 during refresh; does initial group population own the same temporary references and release them after both build passes (including the empty branch)? Core uses that literal shared public-provider contract in both paths, separately from saved-pair and slider references. Please confirm the initial path's list ownership and release order. [open]

<!-- from: re-budget-window-bw22 (2026-10-24), narrowed remainder of archived BW17 -->
- **BW22 - budget-window-bodies.md §3 neighbour rendering/options:** specify set_comboBox (`0x0077DAB0`) option generation and selection, updateNeighborBudget (`0x00781C90`) row reuse/amount formatting, the +8/+12 words of retained row metadata, and the six temporary enumeration lists' reference balance. §3 already specifies enumeration/counts and combo arithmetic; supply remaining raw interface calls rather than a same-owner refresh callback. [open]

- **BW23 - budget-window-bodies.md §5 individual values and saved floats:** specify per-service public queries and exact float/integer operation order in updateIndividualBudget (`0x00785740`), including the two funding floats stored in each +0x44 restore record, visible-control reuse and final cleanup. Reconcile the malformed initializer continuation `0x00786F62` and rebuild=true call at `0x007875C5`. §5 already settles list ordering, record layout/references and rebuild=false preservation. [open]

- **BW24 - budget-window-bodies.md §6 group refresh remainder:** finish UpdateValues (`0x007898A0`): A item text/amount rendering, B traversal and slider rebuilding with counted department references, and final stale-row cleanup. Preserve the proven A header/item quota distinction; do not generalize it to the unproved B pass. BW20 separately covers initial temporary department-list ownership. [open]

- **BW25 - budget-window-bodies.md §1 setup/widget ownership:** specify property replacement (`0x007795A0`), empty/popup creation (`0x0077BEC0` and button path), SetupWindow/Big/Indiv (`0x0077A480`, `0x0077A6F0`, `0x0077A960`), widget factory/QI failure references and exceptional stale controls; classify the still-unlabelled helpers in §1. GetBudgetSegment (`0x0077A390`) is a resource-backed bitmap-segment helper, not a budget-row provider. Which references survive each failed factory/query? [open]
