| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-at30: six concrete automaton records (docs) | Codex; re-at30 worktree; reserved branch re-at30, detached HEAD | ready; import 2b77fe531, specs 019bb5a68; doccheck 1/1, questions 1/1, diff check passed; ABI false, no seam reservations; game n/a, shadow unmeasured; AT29–AT33 answered | 1. Land docs. 2. Engine lane implements six layouts and repeats blocked census/action cases. |

Date: 2026-10-26.
Imported automata-cities29's only docs change without importing any engine changes.
Renumbered its AT24/25/26/27/28 to AT29/30/31/32/33 because main already used AT24/25.
automata.md §§6.4.1–6.4.4 give ordered fields, versions, inheritance, reference IIDs and failure rules.
Carjacked/limo: version 1 plus Vehicle; Ambulance: version 1 plus Emergency.
FireTruck: version 2 plus Emergency and crew ref; explicit changed-crew AddRef/store/Release specified.
Tank: version 2 plus Vehicle and two signed model-index bytes, correcting the two-dword claim.
Tank accepts versions 0/1 without the tail; after accepted framing its result tests stream error alone.
MassTransitTrain: version 1 plus Train/TrainCar versions 2/2, Vehicle and the complete train tail.
Worked cases cover six classes, byte lengths, IIDs, historical versions, null refs and rejection offsets.
Evidence: main checkout's read-only export queries, vtables, decomp where available and instruction bytes.
REA was not needed; no ABI rows proposed and no C ABI exports changed.
Questions archived in questions/answered/automata.md; regenerated QUESTIONS.md, next AT34.
Validation: tools/doccheck.py check and tools/questions.py check passed; git diff --check passed.
The q.py gate wrapper failed to launch Git Bash (CreateFileMapping error 5); direct PowerShell checks passed.
No game run or engine test was performed; monthly shadow differences have not been remeasured.
All five record questions are statically settled; no runtime layout observation remains.
