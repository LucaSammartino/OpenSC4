| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacycontrol4: new spec docs/specs/legacy-control.md for owner cGZLegacyControl (38 of 57 fns specified) | Claude / .claude/worktrees/re-legacycontrol4 | ready (docs only) | LC4: specify the 18 open per-type interpreters, largest first (TreeView 4857, FileBrowser 3761, Grid 3537) |

## Report
- New docs/specs/legacy-control.md: object layout (0x20 bytes, three sub-objects), both slot tables with contracts,
  lifecycle (ctor vocabulary bootstrap in fixed order, Release-at-count<=1, destructor detach/Shutdown when own-control),
  shared vocabulary + interpreter map (IID -> fn) + class map (class -> IID), SetParameters 15-step build sequence
  (fallback create chain, two interpreter passes around Init/ChildAdd/PullToFront, always returns false),
  SetParent/UseWinProc/ForceControl/DoExceptions/GetScriptFragment, property-list readers (type 6 / 0x800E rules),
  every word and ID of all 21 ControlVocabulary functions (asm-extracted, build-runs/vocab2.py), the common
  interpreter's two passes (flag defaults, winflag words, id, area/pos/size, fillcolor, caption/captionres,
  transparent, 8-slot comments list), and the Slider interpreter.
- Measured: 38 of 57 owner functions specified; GetScriptFragment partial; 18 interpreters open.
- Quirks recorded: SetParameters leaks a previous control (Q1), returns false always (Q2), stores the requested clsid
  after a fallback create (Q3), Release at count 0 destroys (Q4).
- Questions: docs/specs/questions/legacy-control.md LC1, LC3-LC6.
- REA not used (export + decomp sufficed). No ABI rows proposed. No engine/ or game runs.
