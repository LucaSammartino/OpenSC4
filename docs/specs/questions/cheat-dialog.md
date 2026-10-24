# cheat-dialog: open questions
<!-- Questions about [cheat-dialog.md](../cheat-dialog.md). Append new entries at the END; when one is answered, move it to answered/cheat-dialog.md. Rules: README.md. -->

<!-- from: re-cheatdlg7 (2026-10-23) -->
- **CCD1 - §3:** message type `0x11` moves the focus to the text edit before the base handling. Which window
  message is `0x11` (activation, or shown)? [inferred]

<!-- from: re-cheatdlg7 (2026-10-23) -->
- **CCD2 - §3:** after a run cheat, the window's keyboard (`D+8`) slot 3 with `0x11` and `0x10` keeps the dialog
  open. Is slot 3 a held-key test with Windows virtual keys (Ctrl, Shift)? [inferred]

<!-- from: re-cheatdlg7 (2026-10-23) -->
- **CCD3 - §1, §4.4:** the sound IDs `+0x114` and `+0x118` stay 0, so the dialog never plays a sound. Did a
  build set them (for example from the creator `0x004E0BB0`), or are the sounds simply unused? [unknown]
