# mysim-tools: open questions
<!-- Questions about [mysim-tools.md](../mysim-tools.md). Append new entries at the END; when one is answered, move it to answered/mysim-tools.md. Rules: README.md. -->

<!-- from: re-mysimtools7 (2026-10-23) -->
- **MST1 - §3.1, §3.2:** what are `cISC4BuildingOccupant` slots 9 and 11, whose pointed-to values the tools test
  (a float ≥ 1.0; a value equal to, or not equal to, 1)? The spec reads them as "fully built" and "residential
  purpose". [inferred]

<!-- from: re-mysimtools7 (2026-10-23) -->
- **MST2 - §3.1, §3.2:** the lot slots the tools read: 41 (a 1–3 value used as the wealth), 33 (must be 3), 35
  (must be below 4) and 63 (called with the Sim's wealth, must be above 0). Which `cISC4Lot` methods are they?
  [inferred]

<!-- from: re-mysimtools7 (2026-10-23) -->
- **MST3 - §5.3:** pylon slots 7 and 20: the filter needs slot 7 ≠ 0, and slot 20's result must QI to `0x0A1DA42F`
  (read as the owning Sim). Are they a UI-state getter and `GetPylonOwner`? [inferred]
