| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrecords-mutations4: LR10/LR11 native mutation contracts | Codex / `re-lotrecords-mutations4` (RE docs) | in progress | 1. Trace setters, wealth update and copy constructor. 2. Publish rules and independent vectors. |

## Handoff
- Goal: answer only LR10/LR11; specs/questions/docs only, no engine reads or edits.
- Worktree reserved and detached; initial status clean, no commits beyond main.
- Done: first spec milestone adds §3.2.1/3.2.2 and T31–T43 native independent vectors (this commit).
- Next: finish known-profile occupancy arithmetic, power-update dependencies and queued-copy reference balance; then reconcile LR10/LR11 questions.
- Proven: SetPopulation clamps unsigned u16 to slot52 AX; selects first profile capacity-map index; AL explicitly 0/1; no map append. SetState snapshot follows power update, not entry.
- Proven: SetState changed path has no boolean return; old wealth for message sampled immediately before outer wealth update, after nested population setter.
- Remaining: runtime initializer for wealth-comparison global 00B49114 (PE zero-fill, xrefs query empty); GetOccupancyByType exact arithmetic and property-copy/release helpers.
- Native addresses: SetPopulation 006C1940, wealth update 006C1430, copy constructor 006C2450.
- q.py cannot launch Bash here (E_ACCESSDENIED); use bounded PowerShell and re_query directly.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-lotrecords-mutations4-r1-manifest.json.
- No game requests or ABI proposals; no LR8/LR9 re-investigation.
