# legacy-control: open questions
<!-- Questions about [legacy-control.md](../legacy-control.md). Append new entries at the END; when one is answered, move it to answered/legacy-control.md. Rules: README.md. -->

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC4 - section 9:** effect tables for the 18 open per-type interpreters (Text, TextEdit, ListBox, Combo, Btn, BMP, Scrollbar, LineInput, Spinner, Outline, TextTicker, SC3WinGen, OptGrp, Folders, FlatRect; TreeView, FileBrowser, Grid done 9.2/9.3/9.5). [unknown; words are registered (section 4.2) but have no effect yet]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC5 - section 6:** is cIGZWinMgr slot 3 (+0x0C), used as the default parent in SetParameters step 11, the root/main window? [inferred: yes]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC7 - section 9.5:** exact branch nesting of the GZWinGrid `wingridcell` flags (0x40000/0x2000/0x10000/0x1000) and the meaning of cell fields 7-11 (parser 0x00956464). [inferred from pseudocode; see 9.5 step 11]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC8 - section 7.6:** what words does each of the 19 per-type script-fragment writers (0x0095F4EC ... 0x009647FB) append after the common words? [unknown; write none until specified]
