# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-budgetwindow-answers4 (2026-10-21) -->
- **BW11 - section 4.0 Alt+Enter / Alt+F4 delivery (runtime):** statically, modifier bit `0x4` is Alt (`GetKeyState(VK_MENU)` in `FUN_0098B6E0`) and `WM_SYSKEYDOWN` reaches the same type-5 builder as `WM_KEYDOWN`, so the dialog closes on Alt+Enter / Alt+F4. In the running game, with a sub-budget dialog open: does Alt+Enter close it (and nothing else happen), and does Alt+F4 close only the dialog or also start the application quit (`cSC4App::RequestQuit`) or a system close? [open: needs a game run; the translator's hook loop at `0x0098D2B6` and keyboard accelerators were not traced]

<!-- from: re-budgetwin (2026-10-22) -->
- **BW18 - budget-window-bodies.md §8, §10, the panel's summary parts:** the construction (`0x007658A0`) and
  `InitBalanceSheet` (`0x00764DF0`) labels, the rest of `Init` (`0x00766E10`), the loan slider's limits in
  `DisplayLoanWindow` (`0x00765C30`) and the row drawing of `UpdateLoanDisplay` (`0x00766680`). [summary]

<!-- from: core-budget-window5 (2026-10-23) -->
- **BW19 - budget-window-bodies.md §3.2 and §6 invalid raw inputs:** what safe replacement behavior is required when combo-edit signed division has a zero divisor or INT_MIN/-1, or UpdateValues receives an empty budget-group vector? The literal native operations would fault. Core returns InvalidDivision for the affected row (continues matching rows and records the single pending neighbor refresh), or None for the empty group vector while clearing its busy guard. These are explicit safe divergences, not successful empty rows. [open]

<!-- from: core-budget-window5 (2026-10-23) -->
- **BW20 - §4 group initialization / budget-window-bodies.md §6 raw department list references:** §6 explicitly releases each department returned by budget slot 31 during refresh; does initial group population own the same temporary references and release them after both build passes (including the empty branch)? Core uses that literal shared public-provider contract in both paths, separately from saved-pair and slider references. Please confirm the initial path's list ownership and release order. [open]

<!-- from: re-budget-window-bw22 (2026-10-24), narrowed remainder of archived BW17 -->
- **BW22 - budget-window-bodies.md §3 neighbour rendering/options:** specify set_comboBox (`0x0077DAB0`) option generation and selection, updateNeighborBudget (`0x00781C90`) row reuse/amount formatting, the +8/+12 words of retained row metadata, and the six temporary enumeration lists' reference balance. §3 already specifies enumeration/counts and combo arithmetic; supply remaining raw interface calls rather than a same-owner refresh callback. [open]

- **BW23 - budget-window-bodies.md §5 individual values and saved floats:** specify per-service public queries and exact float/integer operation order in updateIndividualBudget (`0x00785740`), including the two funding floats stored in each +0x44 restore record, visible-control reuse and final cleanup. Reconcile the malformed initializer continuation `0x00786F62` and rebuild=true call at `0x007875C5`. §5 already settles list ordering, record layout/references and rebuild=false preservation. [open]

- **BW24 - budget-window-bodies.md §6 group refresh remainder:** finish UpdateValues (`0x007898A0`): A item text/amount rendering, B traversal and slider rebuilding with counted department references, and final stale-row cleanup. Preserve the proven A header/item quota distinction; do not generalize it to the unproved B pass. BW20 separately covers initial temporary department-list ownership. [open]

- **BW25 - budget-window-bodies.md §1 setup/widget ownership:** specify property replacement (`0x007795A0`), empty/popup creation (`0x0077BEC0` and button path), SetupWindow/Big/Indiv (`0x0077A480`, `0x0077A6F0`, `0x0077A960`), widget factory/QI failure references and exceptional stale controls; classify the still-unlabelled helpers in §1. GetBudgetSegment (`0x0077A390`) is a resource-backed bitmap-segment helper, not a budget-row provider. Which references survive each failed factory/query? [open]
