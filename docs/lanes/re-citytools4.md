| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-citytools4: city tools and input spec (view input controls) | `re-citytools4` / (RE lane) | in progress | see Handoff |

## Handoff
- Goal: spec docs/specs/city-tools-input.md for the view-input-control family (base cSC4ViewInputControl, Dispatch, the largest tool controllers), lifecycle, event dispatch, activation, interfaces.
- Done: worktree created (base main d09eff17).
- Next: read base cSC4ViewInputControl + Dispatch in decomp/, pick tool owners.
- Facts: per-owner bytes (coverage.py --owners): Query 76565/216 fns, PlaceLot 21780/42, PlayerDrive 12400/40, NetworkToolUI 8005/41, Dispatch 3367/16, base 592/17.
