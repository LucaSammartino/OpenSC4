# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

<!-- from: re-mysim-final-gaps4 (2026-10-22), native CheckCommute request audit -->
- **MS16 - section 4.7 / 7.2:** CheckCommute leaves destination flags, travel-type masks, starting costs,
  maximum trip length and maximum work time untouched. What exact configuration does traffic
  `CreatePathFinder` slot 6 (`0x0070D860`) leave in the returned object (constructor plus Init/factory
  overrides)? [unproved; inherit the returned object, no invented numeric defaults]. Constructor bytes
  at `0x006D89C0` alone do not establish the factory result. The static export does not decode the factory,
  its decomp file lacks it, re_query's raw-disassembly fallback lacks capstone, and REA returns
  `target_unavailable: No app is open`. Resolve from factory/Init bytes before asserting numeric defaults.
