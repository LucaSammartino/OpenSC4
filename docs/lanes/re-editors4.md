| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-editors4 (RE spec: resource and exemplar editor, cSCPropertyView + value editors + edit-value dialog) | `re-editors4` / (RE spec lane) | ready: resource-exemplar-editor.md rounds 1-3 (round 3: cbff4988, d32f7c46, 3690b34d) (round 2 commits 27651b80, e3192315, 14c28703, c7da5d1d); no ABI, no questions, no game runs; export + decomp + exe bytes only, REA not used | 1. coverage-map: `spec` for cSCPropertyView, 3 free functions, cSCEditValueDlg, cSCUint32ComboBoxEntryView, cSCUint8ComboBoxEntryView (exact owners, no wildcard); 2. round 3: grid, colour, curve, spinner, res-key table editors (spec §13) |

## Report

Round 1 (d7119ab5, 7ec03324, a71f9db2): Ingred.ini, property view, type codes, editor class IDs, typed text editors.

Round 2 (this run, docs/specs/resource-exemplar-editor.md only):

- §9 `cSCEditValueDlg` (27651b80): Init 0x00667320 (undecoded by Ghidra, read from instructions): buttons
  Okay 0x26C10A3B / Cancel 0x26C10A3C / Add Value 0x27C32AA4 / Remove Value 0xC7C32AA9 laid out right to left,
  5 px margins; Add/Remove only for variable-length lists (helper 0x0066E470: count < 1 and editor class not
  0xE8DDA013); help label; editor area (5,5)-(W-5, row top). doButtonPress 0x00666D00: Okay writes editor value to
  the property and ends with 1, Cancel ends with 0, Add appends a zero element, Remove drops the last (needs >= 2),
  both rebuild the editor in place; SINT64/STRING/code 8 give an empty value (quirk, jump tables 0x006672D0/F8 read).
  GZOnCommand forwards message 0x287259F6. Corrected §8: class 0xA73B81F0 is a 16-byte helper object, not this dialog.
- §10 combo editors (e3192315): 0x87C34EDA = cSCUint8ComboBoxEntryView proven (vtable 0x00AB0428 slot +0x10 =
  Init entry 0x0068AE30 calling its createComboBox). Item list from the value map (key = value; §3.3 tag removed),
  child IDs 0x46C60DBF / 0xC7C34B1A (+i), layout (5, 5+20i), 18 px high with 5 px overhang (quirk), selection rule,
  GetValue 0x00687AE0 / 0x00687CA0, 8-bit counters in the UINT8 editor (quirk).
- §3.4 (14c28703): ExemplarFilters 0x00672CD0 (key -> filter mask, map 0x00B46924); ResKeyTableEditorDefaults
  0x00671140 (property ID -> four UINT32, stored with v1/v2 swapped, map 0x00B46930, fed to editors 0xA7847D6A /
  0x281F7625 through interface 0x0787ECC4); §6 cross-reference updated.
- §13/§14 (c7da5d1d): open points and honest owner map. §6 property view and free-function contracts unchanged.

Owner map: specified = cSCPropertyView (17843 B), 3 free functions (5164 B, item parsing inferred), cSCEditValueDlg
(1650 B named + ~1165 B Init; folded GZPaint not read), both combo classes (1177 + 1133 B named + 2 GetValue);
typed text editors partly; grid, colour, curves, spinner, res-key table editors and catalog views identity only.
No whole-wildcard credit proposed.

Inferred points: who opens cSCEditValueDlg, reader of 0x00B46924, ref ownership on editor rebuild, combo colour
slot roles, meaning of the ResKeyTableEditorDefaults values.

Questions: none filed. ABI: none proposed. REA: not used (export, decomp and exe bytes via build-runs/rd.py).

## Round 3 report

- §11.1 spinner (cbff4988): range 0..255 quirk, IDs 0xBA293DA4+i, GetValue 0x00689940.
- §11.2 16-column grid (cbff4988): rows = count/16, %u/%d/%.3g cells, in-place truncating write-back, float 3-digit rounding quirk.
- §12 res-key table (d32f7c46): SetValue/GetValue/Save/Revert/Refresh slots, Add/Edit/Delete/Save Table buttons, picker 575x475, words 0/1 swap; 0x67336285 proven cSCResKeyTable (§8 corrected).
- §11.3 colour sequence (3690b34d): pairs, Add/Delete/Refresh, Pick color, GetValue from vector only (typed edits need Refresh).
- §13/§14 updated: curve editors (0x071E971B, 0xE8DDA013) and catalog views remain identity only; no wildcard credit.
REA unavailable (no app open); export + capstone (build-runs/cdis.py) used. No questions, no ABI rows, no game runs.
