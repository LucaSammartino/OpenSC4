# occupant-manager: open questions
<!-- Questions about [occupant-manager.md](../occupant-manager.md). Append new entries at the END; when one is answered, move it to answered/occupant-manager.md. Rules: README.md. -->

<!-- from: re-occfilters (2026-10-20) -->
- **OM9 - §13.2.3, §13.2.4, §13.2.6:** what are the occupant groups `0x4217`, `0x4208`, `0x421D` (excluded by the
  default filter, view level 2) and `0x5001` (hidden by the realistic filter), and the properties `0x49512A09` and
  `0x8A15F1BD` (both exclude from the default filter; levels 3 and 2), `0xCAA9AB92` (not queriable), `0xE83A081D`
  (passed to the model lookup) and `0x29F013E6` (makes an occupant movable for terrain levelling)? The spec uses the
  raw IDs. [as read]
<!-- from: re-occfilters (2026-10-20) -->
- **OM10 - §13.2.3:** which cheat word reaches the 3D view's code at `0x007F365D` that installs a realistic filter
  with all three flags clear? [as read]
