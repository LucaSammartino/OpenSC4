# legacy-control: answered questions

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC5 - section 6:** is cIGZWinMgr slot 3 (+0x0C), used as the default parent in SetParameters step 11, the root/main window? [inferred: yes]
  **ANSWERED 2026-10-23 (re-legacy-control5b):** MATCHES — the manager's current main/root window, also returned by cGZWin::GetMainWindow; borrowed result, replaceable through manager slot 4, not necessarily the initially created base window. See section 6.
<!-- from: core-legacy-control4 (2026-10-22) -->
- **LC9 - sections 7.5, 8, 9:** what portable behavior is required for non-UTF8 string bytes and overflowing CRT decimal/hex scans? [choice: typed String readers/interpreters support valid UTF8; interpreters reject invalid UTF8 and numeric overflow explicitly, without pretending native garbage is reproducible; area unparsed fields 0, first invalid TreeView position -3, Folders unparsed image fields 0, Outline unparsed node components 0 as allowed by the spec]
  **ANSWERED 2026-10-23 (re-legacy-control5b):** MATCHES — section 7.5 adopts valid UTF-8 with byte preservation and explicit unsupported outcomes for invalid bytes or conversions outside the destination range. Documented partial-scan defaults and wrapping arithmetic on valid inputs remain intact. This is portable policy, not a native UTF-8/overflow claim.
<!-- from: core-legacy-control4 (2026-10-22) -->
- **LC10 - sections 9.2, 9.5:** what are duplicate TreeView map-key semantics, and Grid cell resource values before a successful scan when style-word scratch is uninitialized or inhibits the scan? Is the Grid notifier descriptor text ever set by text cells, and what exact arguments do axis notifier helpers copy? [choice: reject duplicate TreeView map keys and uninitialized Grid scratch/resource domains; text-cell notifier descriptor keeps its published fresh empty text; typed axis helper receives the three parsed numbers, with column/row orientation selected by core]
  **ANSWERED 2026-10-23 (re-legacy-control5b):** DIFFERS — section 9.2 requires first-insert-wins unique metadata maps, without suppressing later widget effects or rejecting duplicates; string keys compare full bytes case-sensitively. Section 9.5 retains empty notifier text for text cells and all three scanned axis numbers in oriented records. Undefined Grid scratch/key consumers are explicitly unsupported; defined carry-over remains valid.
<!-- from: re-legacy-control5 (2026-10-23) -->
- **LC11 - sections 9.11, 9.14-9.20, 13.3:** what portable policy should cover TextEdit outlinecolor without a numeric outline, partial unchecked image/resource/gutter/colour scans, OptGrp invalid option IDs/styles and reused image-key scratch, and allocation failures? [unknown native deterministic values; choice: report unsupported edge inputs rather than inventing successful setter arguments or rollback; defined-input effect tables remain normative]
  **ANSWERED 2026-10-23 (re-legacy-control5b):** MATCHES — section 13.3 makes unsupported edge outcomes mandatory for indeterminate unchecked arguments and allocation failure; no successful fabricated setters or transactional rollback promised. Complete in-range unchecked tuples and valid option selectors are the portable domain; existing defined-input effects remain required.
<!-- from: re-legacy-control5 (2026-10-23) -->
- **LC12 - sections 9.20, 13.1:** what messages, return values and ownership rules belong to SC3WinGen's default procedure object (vtable 0x00AD53EC), installed through generator +0x1C before a caller-supplied UseWinProc procedure? [known installation order and initial count 0; callback behavior belongs to the external procedure/generator owner and remains unspecified]
  **ANSWERED 2026-10-23 (re-legacy-control5b):** DIFFERS — section 13.1 specifies six procedure slots: QI accepts 1/0x22E85D8E, structured callback false unchanged, scalar callback true only for 0xA2BF8AD0/0xA2BF8AD1. Generator retains the installed procedure, sends detach/attach notifications on pointer changes, releases replaced procedures, and avoids all effects for same-pointer replacement. Default count starts at zero and reaches one on installation.
<!-- from: re-legacy-control5 (2026-10-23) -->
- **LC13 - section 9.9:** what exact window parameter payload does String-font helper 0x0094C255 store, and does the receiving window apply a font indirectly? [unknown helper internals; owner String-font branches call the helper without a direct font-object/ID setter; do not invent such a setter]
  **ANSWERED 2026-10-23 (re-legacy-control5b):** DIFFERS — sections 9.4/9.9 specify copied String bytes/length in a type-0x800E variant under 0xFAA3BE85 + selector; base cGZWin::SetParam only stores/retains the variant, without font resolution or application. Overrides remain foreign behavior.


<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC6 - section 9.1:** contract of helper 0x0094C1DA (SetGZWinImage, 4 stack arguments) called after an image is set. [unknown]
  **ANSWERED 2026-10-21 (re-legacycontrol4):** DIFFERS — it stores {group, instance} as an 8-byte type-0x8000 variant in control parameter 0xDAA3B860 + n (Slider n = 0); see section 9.4.
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC1 - section 1:** the +0x04 sub-object vtable 0x00AD5344 (IID 0x5A356159) has no slot targets in the export; what are its slots? [unknown; implement QI/AddRef/Release thunks only]
  **ANSWERED 2026-10-21 (re-legacycontrol4):** DIFFERS — 8 slots read from the image: QI/AddRef/Release this-4 thunks, two 1-arg return-true stubs, SetArea/GetArea forwarders to the control (false when no control), one 2-arg return-true stub; see section 3.
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC3 - section 7.4:** what text does serialiser 0x00965F73 (called by GetScriptFragment) write for a control? [unknown; return false until specified]
  **ANSWERED 2026-10-21 (re-legacycontrol4):** DIFFERS — it writes `<LEGACY clsid=.. iid=.. ` + common words (id, area, fillcolor, caption, captionres, comments, 13 yes/no flags) + per-type words + `>
` and always succeeds; see section 7.6 (per-type word sets open as LC8).
<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC7 - section 9.5:** exact branch nesting of the GZWinGrid `wingridcell` flags (0x40000/0x2000/0x10000/0x1000) and the meaning of cell fields 7-11 (parser 0x00956464). [inferred from pseudocode; see 9.5 step 11]
  **ANSWERED 2026-10-21 (re-legacycontrol4):** DIFFERS — 0x40000 text branch excludes image/string-resource; 0x2000 image else 0x10000 string resource; 0x1000 record independent; the key scan tests the stale style-word scratch (Q10); record offsets in section 9.5 step 11.

<!-- from: re-legacycontrol4 (2026-10-21) -->
- ~~**LC4 - section 9:** effect tables for the 12 open per-type interpreters (Btn, OptGrp, TextEdit, Spinner, LineInput, ListBox, SC3WinGen, Combo, TextTicker, Text, Scrollbar, BMP; TreeView, FileBrowser, Grid, Folders, FlatRect, Outline done 9.2/9.3/9.5-9.8). [unknown; words are registered (section 4.2) but have no effect yet]~~
  **ANSWERED 2026-10-23 (re-legacy-control5):** DIFFERS — defined-input effects are specified in legacy-control.md sections 9.9–9.20, including per-pass setter offsets, exact-one versus nonzero flags, resource-load failure effects, ordered repeated properties and numeric alpha behavior. Unchecked-input/allocation domains are isolated in LC11; external generator procedure semantics in LC12; String-font helper payload in LC13.
