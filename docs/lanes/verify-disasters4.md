| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-disasters4 (verify the non-exact disasters_shadow: fire 211/263, riot 2/354) | `verify-disasters4` / (engine lane) | in progress | - |

## Handoff
- Goal: find the first primary mismatch of disasters_shadow (fire first) under a scripted disaster, fix if spec-supported else file question.
- Done: worktree at main 4726020a.
- Next: read FC4DisasterShadow.cpp compare paths, find the original script/run that gave fire 211/263.
