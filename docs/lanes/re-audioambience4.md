| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioambience4 (RE spec: audio ambience AA2/AA1) | `re-audioambience4` / (RE lane) | in progress | AA2 then AA1 |

## Handoff
- Goal: answer AA2 (0x004E9DA0 condition change) then AA1 (ambience layer 0x004E5110-0x004E8230) in docs/specs/audio-ambience.md; AA3/AA4 if cheap.
- Done: AA2 answered in spec 3.4 (71582dec); AA1 part 1: layer Init + tuning table, spec 8.1-8.2 (917e998f); AA1 part 2 filters 8.3 + AA3 answered (commit 3).
- Next: AA1 part 3: CanHandle 0x004E5110, HandleCell/HandleOccupant, scores 0x004E5920, loops 0x004E5D50, Update 0x004E6340. REA unavailable; decomp + build-runs/stk.py (linear ESP tracker, crude).
