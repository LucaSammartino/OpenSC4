| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-editors4 (RE spec: resource and exemplar editor, cSCPropertyView + entry views) | `re-editors4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: new docs/specs/resource-exemplar-editor.md for the 68985-byte "resource and exemplar editor" subsystem; focus cSCPropertyView + cSC*EntryView + cSCEditValueDlg + free fns (largest coherent family).
- Done: worktree; spec v1 docs/specs/resource-exemplar-editor.md (Ingred.ini sections, Properties record, type codes, view build, class-ID table) commit SPEC1.
- Next: round-1 depth on cSCStandardTypeEntryView text editors (Init/GetValue 0x00688xxx), assignValuesToProperty 0x0066EC80, GZOnCommand 0x0066F740; then finish report + merge-queue.
- REA unavailable (no app open); export/decomp only. build-runs/va.py dumps exe bytes at a VA; refs.py lists string refs per owner.
- Facts: bytes: cSCPropertyView 17843, cSCResourceCatalogView 11099, cSCResKeyTableEntryView 5899, GeneralResponseCurve 5525, ResKeyInputView 4374, fn:getDefaultValue 4179, HappinessCurve 3586, 16ColumnGrid 3079, ColorSequence 1821, EditValueDlg 1593, StandardType 1340, Uint32Combo 1250, Uint8Combo 1133, ResCatBaseView 1005, small text entry views ~430 each.
