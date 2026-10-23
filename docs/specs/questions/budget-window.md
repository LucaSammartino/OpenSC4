# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-budgetwindow-answers4 (2026-10-21) -->
- **BW11 - section 4.0 Alt+Enter / Alt+F4 delivery (runtime):** statically, modifier bit `0x4` is Alt (`GetKeyState(VK_MENU)` in `FUN_0098B6E0`) and `WM_SYSKEYDOWN` reaches the same type-5 builder as `WM_KEYDOWN`, so the dialog closes on Alt+Enter / Alt+F4. In the running game, with a sub-budget dialog open: does Alt+Enter close it (and nothing else happen), and does Alt+F4 close only the dialog or also start the application quit (`cSC4App::RequestQuit`) or a system close? [open: needs a game run; the translator's hook loop at `0x0098D2B6` and keyboard accelerators were not traced]

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
  summary parts are BW18.)
  See companion sections 1 and 7 for the full inventory; these remain questions, not accepted host inputs.

The original request follows.
- **BW17 - sections 1, 4, 4.1, 5.2, 5.6 and 8 whole-owner inventory / omitted bodies:** please publish the full method/slot inventory for `cSC4SubBudgetWindow` and the owned budget-panel paths, and the non-layout algorithms still omitted: ordinance enumeration/filtering, A/B counts and row-map construction in `initOrdinanceBudget`; proposed-deal enumeration, section/count rules and `+0x50` selection edits in `initNeighborBudget`/combo handling; occupant enumeration and the exact creation/ownership of `+0x44` individual restore records; all four per-mode refreshes (including group row reuse on scroll), empty/popup window creation and failure reference rules, and the panel loan-display/balance-sheet bodies. Which public external interface calls and arguments supply their raw inputs? [unimplemented: existing same-class populate/refresh host callbacks remain explicit gaps, not core coverage; no native own-class result may count as an external input. BW14/BW15 ownership and BW16 visibility are implemented from their settled text; the documented BW13 null-content divergence is retained. No whole-owner completion claimed.]

<!-- from: re-budgetwin (2026-10-22) -->
- **BW18 - budget-window-bodies.md §8, §10, the panel's summary parts:** the construction (`0x007658A0`) and
  `InitBalanceSheet` (`0x00764DF0`) labels, the rest of `Init` (`0x00766E10`), the loan slider's limits in
  `DisplayLoanWindow` (`0x00765C30`) and the row drawing of `UpdateLoanDisplay` (`0x00766680`). [summary]
