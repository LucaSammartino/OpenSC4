# budget-window: open questions
<!-- Questions about [budget-window.md](../budget-window.md). Append new entries at the END; when one is answered, move it to answered/budget-window.md. Rules: README.md. -->

<!-- from: re-panels4 (2026-10-21) -->
- **BW5 - section 4 key close:** key messages `0x0D` and `0x73` close the dialog only when modifier bit `0x4` is set. Is `0x73` the virtual key F4 (Alt+F4) and bit `0x4` Alt, and does plain Enter really do nothing? [yes to F4 by key value; plain Enter returns 0 with no effect (byte-checked at `0x0078BCF0`, re-budgetwindow4); the meaning of bit `0x4` is not in the dialog code and needs a game check]

<!-- from: core-budgetwindow-fix4 (2026-10-21) -->
- **BW10 - sections 4, 4.1 remaining on_init contracts:** (a) who sets `+0x10` (released by `on_shutdown` and the destructor)? (b) group mode with both counts 0 (empty window): are the `SliderInfo` rows (`+0x2C`) and the saved fractions (`+0x38`) still built, so a later Cancel restores them? (c) which state does `on_init` give the four scroll arrows `0x451`-`0x454` ("on_init sets their initial state", §5.3)? [core: (a) holds nothing at `+0x10`; (b) builds neither; (c) sets no arrow at init]
