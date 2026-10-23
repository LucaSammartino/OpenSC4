| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotcfg-editor5: lot configuration editor windows | Codex / re-lotcfg-editor5 | in progress | Map editor and chooser owners; write spec and vectors. |

## Handoff
- Goal: specify cSC4WinLotConfigurationEditor and cSC4WinLotConfigurationChooser using local export only.
- Worktree is reserved and HEAD detached; commit normally, do not change refs.
- Done: read rules; found existing 18-section editor spec and exact inventory. Extend rather than replace.
- Existing LE3/LE4/LE5 are parked after prior rounds; preserve explicit unknowns and historical counts.
- Chooser source: main decomp/cSC4WinLotConfigurationChooser.c; unnamed helpers in decomp/_unnamed/region_00470000.c.
- Measured PE tables: editor 0x00A8D888 (160 slots), chooser 0x00A8D4E0 (152 slots); helper build-runs/inspect-tables.py.
- Next: finish chooser lifecycle/dispatch/list/edit/delete contract; add table map and editor integration sections.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-lotcfg-editor5-r1-manifest.json.
- No game requests or ABI changes planned.
