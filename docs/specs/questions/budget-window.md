# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-panels4 (2026-10-21) -->
- **BW4 - section 2.1 panel slot 6:** commands `0xAA3AC40E`/`0xAA3AC40F` (and `0x4A8879CB`/`0x4A8879DB`) set `+0x18C` to 1/0 and call the budget proc's slot 6 (`+0x18`). Which method is it (ShowWindow with expanded/compact)? [inferred: switch between the compact and expanded panel]

<!-- from: re-panels4 (2026-10-21) -->
- **BW5 - section 4 key close:** key messages `0x0D` and `0x73` close the dialog only when modifier bit `0x4` is set. Is `0x73` the virtual key F4 (Alt+F4) and bit `0x4` Alt, and does plain Enter really do nothing? [yes to F4 by key value; modifier meaning unverified, needs a game check]

<!-- from: core-budgetwindow4 (2026-10-21) -->
- **BW6 - sections 2, 3, 6 references and pause balance:** (a) `CreateSubBudgetWindow` AddRefs the `*out` window and also holds it at `+0x0C`: who releases each (the panel after step 5, `DestroySubBudgetWindow`, the deleting dtor)? (b) `DestroySubBudgetWindow` calls `HiddenResume` "in every mode": also when `CreateSubBudgetWindow` failed and `on_init` (the `HiddenPause`) never ran, or is the resume guarded? [core implements: no Release of either; resume always]

<!-- from: core-budgetwindow4 (2026-10-21) -->
- **BW9 - sections 4, 5 unspecified routing:** what `on_shutdown` (`0x0077E440`) does; `DoWinMsg`'s return value for init, shutdown, slider, combo and ignored keys; the meaning of the second argument `0x42B7C353` of `end_dialog` on the Esc path (section 6 gives one argument); which code creates the inner child at `+0x14`; the ordinance row check IDs that toggle the edited byte; the arguments of `SetOrdinanceOn` in `0xCD` (core: ordinance, edited state). [unknown; core takes child and toggles as typed host input and returns "unspecified"]
