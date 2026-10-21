| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-editors4 (RE spec: resource and exemplar editor, cSCPropertyView + typed value editors) | `re-editors4` / (RE spec lane) | ready: new docs/specs/resource-exemplar-editor.md round 1 (commits d7119ab5, 7ec03324, a71f9db2); no ABI, no questions filed, no game runs; REA unavailable (no app open), export + decomp only | 1. coverage-map: add a `cSCPropertyView` row (spec resource-exemplar-editor.md) and rows for fn:getDefaultValue/getUnquotedString/convertStringToFloatValue ahead of `cSC*View` (tools edit, coordinator). 2. Round 2: curve/colour/grid/combo/res-key-table editors, cSCEditValueDlg::doButtonPress 0x00666D00, ExemplarFilters/ResKeyTableEditorDefaults callbacks. |

## Report

What changed: new spec docs/specs/resource-exemplar-editor.md (no other files besides this one).

- §2–§5: `Sys\Ingred.ini` loading (once per process, global byte 0x00B468F0), its six section kinds and order,
  Aliases, space-separated filter masks, the comma-separated `[Properties]` record (10 fields, tPropertyInfo
  offsets, drop rules: unknown type, STRING with count != 1, default parse failure; count 0 gives an empty uint32
  array), type-name table with cRZVariant codes read from the return instructions of getVariantType 0x0066EE80.
- §6–§6.1: view build (400x450 centred, controls, filter-masked property-to-editor map, GetValueEditor via GZCOM
  IID 0x06C5FF7B), commands: list selection 0xE8711255/0xFA12B77F switches editor and help text; Done 0x24C93018
  writes ID + value into the target property and ends with 0; Cancel 0x24C93019 ends with 1.
- §7: value-to-text formatting rules and diagnostics strings.
- §8: 20 editor class IDs with factories from cSC4ResourceLibCOMDirector ctor 0x00664920; §8.1 typed text editors
  (layout x=5, y=5+20i, child ID bases, true/false text) and the SINT64 scalar quirk (atol + CDQ at 0x0068841B
  truncates to 32 bits; lists use RZAtoi64A).

Owner map (honest, §10): specified = cSCPropertyView (17843 B, minus two callbacks) and the 3 free functions
(5164 B, item-level parse inferred); typed text editors (~3700 B) partly; everything else in the 68985-byte
subsystem (curve/colour/grid/combo/res-key editors, cSCEditValueDlg, catalog views, cSCResKeyInputView) is identity
only. Recommend the coverage-map row only for cSCPropertyView + free functions; leave `cSC*EntryView`, `cSC*View`,
`cSCEditValueDlg` at `-`.

Inferred points flagged in the spec: 0x87C34EDA = Uint8 combo (vtable 0x00AB0428 not in docs/abi), class
0x281F7625 not registered by this director, value-map entry direction, non-modal end path, control/string pairing.

Questions: none filed. ABI: none proposed.
