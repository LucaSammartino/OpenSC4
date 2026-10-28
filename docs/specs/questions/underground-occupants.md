# underground-occupants: open questions
<!-- Questions about [underground-occupants.md](../underground-occupants.md). Append new entries at the END; when one is answered, move it to answered/underground-occupants.md. Rules: README.md. -->

<!-- from: re-underground (2026-10-19) -->
- **UG3 - §3.3:** which subway pieces are `0x300` and `0x4B00` (their tunnel floor comes from the second vertex
  list), and what the zipper piece type (main slot 99) being 0 means for them.
  **PARTIAL 2026-10-27 (re-x-net):** The zipper answer only selects whether the second-list corners are lowered (non-zero: not lowered; always 0 for `0x300`); underground-occupants.md §3.3. Which tunnel shapes pieces `0x300` and `0x4B00` are is decided by the rule data, not the executable.
