| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-loteditor-window4 (RE: native lot configuration editor) | `re-loteditor-window4` / RE spec lane | in progress; detached worktree reused; no ABI or game runs | Inventory owner52646B, specify largest bodies, reconcile all attributed functions and gaps. |

## Handoff

- Goal: native cSC4WinLotConfigurationEditor only; prose specs, no engine/tools/ABI edits.
- Done: 3ce25ffe popup/compatibility milestone; next milestone adds interfaces/lifecycle, new/open/Save/Save As/Close, finite geometry/bounds and mutation quirks, independent vectors; no owner credit.
- Next: write exact inventory companion, reconcile remaining large bodies as budget allows, final report/queue; LE1-LE5 are open.
- Inventory: export 92 named bodies/51054B; fresh coverage-owners-base.tsv attributes 111 entries/52880B (19 extras/1826B); brief label52646B differs234B.
- Critical correction: Save As true entry00485390 in vtable00A8DAE8; label00485445 is inside MOV at00485444. Prefix180B + uncovered1B + labelled700B =881B contiguous workflow.
- Native bytes read with build-runs/loteditor_native.py; capstone unavailable. REA not used; existing pseudo and export available but some stack recovery corrupt.
- Source: main checkout C:/Users/tlori/FreeCity4; export and decomp read-only; edits only here.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell/local Python commands.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-loteditor-window4-r1-manifest.json.
- No whole-owner credit until every attributed body is reconciled; file precise questions for gaps.
