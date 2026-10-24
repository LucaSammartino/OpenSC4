# tutorial-window: open questions
<!-- Questions about [tutorial-window.md](../tutorial-window.md). Append new entries at the END; when one is answered, move it to answered/tutorial-window.md. Rules: README.md. -->

<!-- from: re-tutwin7 (2026-10-23) -->
- **TUW1 - §1:** `+0x18` (the shown arrow) and `+0x2C` (slot 21 `(-1, 0, 0)`) are read and released here but
  written elsewhere. Which code sets them: the tasks, or the message target's `DoMessage` (`0x00444480`)?
  [unknown]

<!-- from: re-tutwin7 (2026-10-23) -->
- **TUW2 - §5.2:** the button interface `0x8810` slot 44 gets `(0x7FFFFFFF, 200)` to highlight and `(0, 200)` to
  stop. Is it a blink count and a 200 ms period? [inferred]

<!-- from: re-tutwin7 (2026-10-23) -->
- **TUW3 - §7.1:** each arrow gets `SetFlag(0x200000, 1)` after `HideWindow`. What does window flag `0x200000`
  do? [unknown]

<!-- from: re-tutwin7 (2026-10-23) -->
- **TUW4 - §4, §8:** `DoSpecialDocumentProcessing` never returns for a text with a `<!--` after its first `-->`.
  Does any shipped tutorial text (the `tutorialtasks` tasks' slot 8 and slot 9 texts) have two comments, or a
  comment after a `-->`? [unknown: needs the shipped scripts]

<!-- from: re-tutwin7 (2026-10-23) -->
- **TUW5 - §8:** the `cSC4TutorialTask` slot uses are read only from this window (3 build, 7 allowed ID, 8 and 9
  texts, 11 and 12 hint window, 14 start, 16 end, 19 value, 20 type, 22 zoom, 23 and 24 offset). Do the task's own
  bodies confirm them, and what are task types 1, 4, 8, 0xB–0xD and 0x10? [inferred]

<!-- from: re-tutwin7 (2026-10-23) -->
- **TUW6 - §5.1:** interface `0x49D98264` gets slot 18 `(-1, 1)` from the reset walker and `(task slot 19, 0)`
  from the hint walker. Which window class has it, and what is the value? [unknown]
