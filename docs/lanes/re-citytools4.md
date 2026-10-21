| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-citytools4: city tools and input spec (view input controls) | `re-citytools4` / (RE lane) | in progress | see Handoff |

## Handoff
- Goal: spec docs/specs/city-tools-input.md for the view-input-control family (base cSC4ViewInputControl, Dispatch, the largest tool controllers), lifecycle, event dispatch, activation, interfaces.
- Done: worktree (base main d09eff17); docs/specs/city-tools-input.md §2 base class full, §3 place-lot lifecycle/input; questions CT1-CT5.
- Next: CT5 (PlaceLot/CanPlaceLot bodies) to claim cSC4ViewInputControlPlaceLot; or base+Dispatch tool.
- Facts: per-owner bytes (coverage.py --owners): Query 76565/216 fns, PlaceLot 21780/42, PlayerDrive 12400/40, NetworkToolUI 8005/41, Dispatch 3367/16, base 592/17.
