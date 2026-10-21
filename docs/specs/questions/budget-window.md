# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-budgetwindow-answers4 (2026-10-21) -->
- **BW11 - section 4.0 Alt+Enter / Alt+F4 delivery (runtime):** statically, modifier bit `0x4` is Alt (`GetKeyState(VK_MENU)` in `FUN_0098B6E0`) and `WM_SYSKEYDOWN` reaches the same type-5 builder as `WM_KEYDOWN`, so the dialog closes on Alt+Enter / Alt+F4. In the running game, with a sub-budget dialog open: does Alt+Enter close it (and nothing else happen), and does Alt+F4 close only the dialog or also start the application quit (`cSC4App::RequestQuit`) or a system close? [open: needs a game run; the translator's hook loop at `0x0098D2B6` and keyboard accelerators were not traced]

<!-- from: re-budget-slider-ownership4 (2026-10-22) -->
- **BW16 - section 4.1 row-visibility ID test direction (static):** §4.1 rule 2 says an item is shown if `GetID() > 100` or its amount is non-zero. The bytes at all four sites (count A `0x007880D3`, count B `0x00788147`, build A `0x007886ED`, build B `0x00788F33`) are `CMP EAX,0x64; JBE <shown>`, then for ID > 100 a 64-bit amount test (`OR EAX,EDX; JZ <hidden>`): i.e. ID ≤ 100 is always shown (subject to display flag bit 0) and ID > 100 is hidden when its amount is zero. Is slot 3 really `GetID`, and should §4.1 (and its worked example) be inverted? [open: needs a reviewer of §4.1; BW15 tests avoid the case]
