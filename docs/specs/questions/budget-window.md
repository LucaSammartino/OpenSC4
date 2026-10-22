# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-budgetwindow-answers4 (2026-10-21) -->
- **BW11 - section 4.0 Alt+Enter / Alt+F4 delivery (runtime):** statically, modifier bit `0x4` is Alt (`GetKeyState(VK_MENU)` in `FUN_0098B6E0`) and `WM_SYSKEYDOWN` reaches the same type-5 builder as `WM_KEYDOWN`, so the dialog closes on Alt+Enter / Alt+F4. In the running game, with a sub-budget dialog open: does Alt+Enter close it (and nothing else happen), and does Alt+F4 close only the dialog or also start the application quit (`cSC4App::RequestQuit`) or a system close? [open: needs a game run; the translator's hook loop at `0x0098D2B6` and keyboard accelerators were not traced]

<!-- from: core-budgetwindow-ownership4 (2026-10-22) -->
<!-- Partial BW17 evidence: re-budgetwindow-bodies4, 2026-10-22. -->
BW17 is partially resolved in [budget-window-bodies.md](../budget-window-bodies.md): exact exported owner/slot
inventory, available-ordinance classification/map construction, six neighbour scans and counts, combo edits,
and CalculateValues raw-input arithmetic. It remains open for the three bounded audit groups in that spec's
section 5; no full-owner completion or same-owner callback boundary is implied. The original request follows.
- **BW17 - sections 1, 4, 4.1, 5.2, 5.6 and 8 whole-owner inventory / omitted bodies:** please publish the full method/slot inventory for `cSC4SubBudgetWindow` and the owned budget-panel paths, and the non-layout algorithms still omitted: ordinance enumeration/filtering, A/B counts and row-map construction in `initOrdinanceBudget`; proposed-deal enumeration, section/count rules and `+0x50` selection edits in `initNeighborBudget`/combo handling; occupant enumeration and the exact creation/ownership of `+0x44` individual restore records; all four per-mode refreshes (including group row reuse on scroll), empty/popup window creation and failure reference rules, and the panel loan-display/balance-sheet bodies. Which public external interface calls and arguments supply their raw inputs? [unimplemented: existing same-class populate/refresh host callbacks remain explicit gaps, not core coverage; no native own-class result may count as an external input. BW14/BW15 ownership and BW16 visibility are implemented from their settled text; the documented BW13 null-content divergence is retained. No whole-owner completion claimed.]
