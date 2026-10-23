# legacy-control: open questions
<!-- Questions about [legacy-control.md](../legacy-control.md). Append new entries at the END; when one is answered, move it to answered/legacy-control.md. Rules: README.md. -->

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC5 - section 6:** is cIGZWinMgr slot 3 (+0x0C), used as the default parent in SetParameters step 11, the root/main window? [inferred: yes]
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC8 - section 7.6:** what words does each of the 19 per-type script-fragment writers (0x0095F4EC ... 0x009647FB) append after the common words? [unknown; write none until specified]
<!-- from: core-legacy-control4 (2026-10-22) -->
- **LC9 - sections 7.5, 8, 9:** what portable behavior is required for non-UTF8 string bytes and overflowing CRT decimal/hex scans? [choice: typed String readers/interpreters support valid UTF8; interpreters reject invalid UTF8 and numeric overflow explicitly, without pretending native garbage is reproducible; area unparsed fields 0, first invalid TreeView position -3, Folders unparsed image fields 0, Outline unparsed node components 0 as allowed by the spec]
<!-- from: core-legacy-control4 (2026-10-22) -->
- **LC10 - sections 9.2, 9.5:** what are duplicate TreeView map-key semantics, and Grid cell resource values before a successful scan when style-word scratch is uninitialized or inhibits the scan? Is the Grid notifier descriptor text ever set by text cells, and what exact arguments do axis notifier helpers copy? [choice: reject duplicate TreeView map keys and uninitialized Grid scratch/resource domains; text-cell notifier descriptor keeps its published fresh empty text; typed axis helper receives the three parsed numbers, with column/row orientation selected by core]
<!-- from: re-legacy-control5 (2026-10-23) -->
- **LC11 - sections 9.11, 9.14-9.20, 13.3:** what portable policy should cover TextEdit outlinecolor without a numeric outline, partial unchecked image/resource/gutter/colour scans, OptGrp invalid option IDs/styles and reused image-key scratch, and allocation failures? [unknown native deterministic values; choice: report unsupported edge inputs rather than inventing successful setter arguments or rollback; defined-input effect tables remain normative]
<!-- from: re-legacy-control5 (2026-10-23) -->
- **LC12 - sections 9.20, 13.1:** what messages, return values and ownership rules belong to SC3WinGen's default procedure object (vtable 0x00AD53EC), installed through generator +0x1C before a caller-supplied UseWinProc procedure? [known installation order and initial count 0; callback behavior belongs to the external procedure/generator owner and remains unspecified]
