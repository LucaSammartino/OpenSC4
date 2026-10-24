# ui-helpers: open questions
<!-- Questions about [ui-helpers.md](../ui-helpers.md). Append new entries at the END; when one is answered, move it to answered/ui-helpers.md. Rules: README.md. -->

<!-- from: re-uihelpers7 (2026-10-23) -->
- **UIH1 - §5:** the global `0x00B43CE0` gets slot 10 `(dialog, 1)` before the modal run and slot 6 `(dialog)`
  after it. Is it a window-location store that restores and saves a dialog's place? Which class is it? [inferred]

<!-- from: re-uihelpers7 (2026-10-23) -->
- **UIH2 - §5:** the dialog texts go through interface `0x212CDC1F` slot 4 and the button labels through the
  button interface `0x8810` slot 49. Are these the caption setters of a text window and a button? [inferred]

<!-- from: re-uihelpers7 (2026-10-23) -->
- **UIH3 - §3:** the catalog properties `0x8A2602BA` (menu), `0x8A2602B9` (order), `0x8A2602B8` (icon),
  `0x8A2602BB` (button ID), `0x8A2602A9` (description), `0x8A416A99` and `0xCA416AB5` (name and description text
  keys) and `0x899AFBAD` (a name). Do the shipped property names confirm these meanings? [inferred]
