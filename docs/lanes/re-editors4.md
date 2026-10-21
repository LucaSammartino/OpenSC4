| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-editors4 (RE spec: resource and exemplar editor, cSCPropertyView + typed value editors) | `re-editors4` / (RE spec lane) | in progress (round 2): docs/specs/resource-exemplar-editor.md round 1 (commits d7119ab5, 7ec03324, a71f9db2); no ABI, no questions filed, no game runs; REA unavailable (no app open), export + decomp only | 1. coverage-map: add a `cSCPropertyView` row (spec resource-exemplar-editor.md) and rows for fn:getDefaultValue/getUnquotedString/convertStringToFloatValue ahead of `cSC*View` (tools edit, coordinator). 2. Round 2: curve/colour/grid/combo/res-key-table editors, cSCEditValueDlg::doButtonPress 0x00666D00, ExemplarFilters/ResKeyTableEditorDefaults callbacks. |

## Handoff

Goal (round 2): extend docs/specs/resource-exemplar-editor.md with curve, colour, 16-column grid, combo, res-key-table
editors; cSCEditValueDlg::doButtonPress 0x00666D00; ExemplarFilters 0x00672CD0 and ResKeyTableEditorDefaults callbacks;
verify 0x87C34EDA Uint8 combo identity. Keep §6 property view and free-function contracts as is.

Done: round 1 d7119ab5, 7ec03324, a71f9db2, report 96a65455.
Done r2: §9 edit-value dialog (Init 0x667320 + doButtonPress) 27651b80; §10 combos + 0x87C34EDA proven (this commit).
Next: §11 16-column grid (decomp/cSC16ColumnGridEntryView.c), §12 colour/curves/res-key table; ExemplarFilters 0x672CD0;
then renumber open points (§13) and owner map (§14). build-runs/rd.py VA:count|s reads exe data.
Facts: round-1 report content (owner map) is in spec §10; no ABI, no questions.
