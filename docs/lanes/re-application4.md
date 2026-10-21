| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-application4: application framework spec, cSC4App lifecycle/services/messages | `re-application4` / (RE spec lane) | in progress (round 2): city request/load/save/close 0x00454370-0x00456910, AP1, AP2 | see Handoff |

## Handoff
- Goal round 2: extend docs/specs/application.md with city request/load/save/close path (0x00454370-0x00456910),
  answer AP1 (ReadCommandLine 0x0044EED0 per-option effects) and AP2 (OnIdle 0x00449C70 300 ms gate/tail).
- Done round 1: spec bf01cf8c, report 0b7971c7 (partial: ~35 of 322 cSC4App functions; sections 1-11).
- Round 2: section 11 (requests, close, region view, obliteration) added; section 12 = open list.
- Next: NewCity 0x004548B0, LoadCity 0x004555F0, RequestLoadCity 0x00456910, DoLoadCityUI 0x00456860, RequestSaveCity 0x00455AE0, SaveCity 0x00454DA0/0x00455E20, RequestQuit 0x004546B0; then AP2 OnIdle, AP1. Tool: build-runs/asmf.py ADDR (filtered asm).
- Facts: REA may be unavailable; export asm is primary evidence. Round 1 report text is in git 0b7971c7.
