# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-budgetwindow-answers4 (2026-10-21) -->
- **BW11 - section 4.0 Alt+Enter / Alt+F4 delivery (runtime):** statically, modifier bit `0x4` is Alt (`GetKeyState(VK_MENU)` in `FUN_0098B6E0`) and `WM_SYSKEYDOWN` reaches the same type-5 builder as `WM_KEYDOWN`, so the dialog closes on Alt+Enter / Alt+F4. In the running game, with a sub-budget dialog open: does Alt+Enter close it (and nothing else happen), and does Alt+F4 close only the dialog or also start the application quit (`cSC4App::RequestQuit`) or a system close? [open: needs a game run; the translator's hook loop at `0x0098D2B6` and keyboard accelerators were not traced]

<!-- from: coordinator review (2026-10-21) -->
- **BW12 - section 5.3, selector-specific initial arrow bounds:** the general group on_init bounds at 0x007896C9/0x00789714 are proven, but initOrdinanceBudget (0x0077D620-0x0077D780) and initNeighborBudget (0x00781AD1-0x00781C4D) were not byte-checked. Which count fields and exact comparisons control each initial down arrow, and do these helpers differ from general group mode? Do not extend the general <=9 rule to these copies without proof. [open: prevents whole-owner credit for guessed selector-specific initialization]
