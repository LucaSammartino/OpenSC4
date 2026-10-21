# legacy-control: open questions
<!-- Questions about [legacy-control.md](../legacy-control.md). Append new entries at the END; when one is answered, move it to answered/legacy-control.md. Rules: README.md. -->

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC1 - section 1:** the +0x04 sub-object vtable 0x00AD5344 (IID 0x5A356159) has no slot targets in the export; what are its slots? [unknown; implement QI/AddRef/Release thunks only]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC3 - section 7.4:** what text does serialiser 0x00965F73 (called by GetScriptFragment) write for a control? [unknown; return false until specified]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC4 - section 9:** effect tables for the 18 open per-type interpreters (Text, TextEdit, ListBox, Combo, Btn, BMP, Scrollbar, LineInput, Spinner, Outline, TextTicker, SC3WinGen, FileBrowser, OptGrp, TreeView, Folders, Grid, FlatRect). [unknown; words are registered (section 4.2) but have no effect yet]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC5 - section 6:** is cIGZWinMgr slot 3 (+0x0C), used as the default parent in SetParameters step 11, the root/main window? [inferred: yes]
