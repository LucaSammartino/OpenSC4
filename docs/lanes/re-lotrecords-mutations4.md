| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrecords-mutations4: LR10/LR11 native mutation contracts | Codex / `re-lotrecords-mutations4` (RE docs) | in progress | 1. Trace setters, wealth update and copy constructor. 2. Publish rules and independent vectors. |

## Handoff
- Goal: answer only LR10/LR11; specs/questions/docs only, no engine reads or edits.
- Worktree reserved and detached; initial status clean, no commits beyond main.
- Done: faa5a656 adds §3.2.1/3.2.2 and initial native vectors; current milestone completes rules and T31–T57, reconciles LR10/LR11 and regenerates index.
- Next: final documentation audit, replace Handoff with report, commit and queue actual detached HEAD.
- Proven: SetPopulation clamps unsigned u16 to slot52 AX; selects first profile capacity-map index; AL explicitly 0/1; no map append. SetState snapshot follows power update, not entry.
- Proven: SetState changed path has no boolean return; old wealth for message sampled immediately before outer wealth update, after nested population setter.
- Resolved: 00B49114 is zero-filled float 0; raw PE reference scan found two reads and no absolute writer. Wealth chooses strict positive maximum descending by wealth; zero scores preserve supported wealth/fallback.
- Resolved: occupancy helper inserts missing population keys, averages capacities using integer division; empty capacity-map element faults at 006C1258. Unit/travel designate both null in snapshot; configuration/property refs balance through Shutdown/destruction.
- Queue proof: MessagePost 0092D0BB -> PushBack 0092DDD2, message-node retain 007E98E5; message owns copy via 005447C7, destructor releases at 009133C9.
- Native addresses: SetPopulation 006C1940, wealth update 006C1430, copy constructor 006C2450.
- q.py cannot launch Bash here (E_ACCESSDENIED); use bounded PowerShell and re_query directly.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-lotrecords-mutations4-r1-manifest.json.
- No game requests or ABI proposals; no LR8/LR9 re-investigation.
