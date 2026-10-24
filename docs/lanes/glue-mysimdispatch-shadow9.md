| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysimdispatch-shadow9: dispatch replay round 2 | Codex / `glue-mysimdispatch-shadow9` (engine, detached HEAD) | in progress | Implement specified contracts and measure house to 2032. |

## Handoff
- Goal: implement MS18/MS20 dispatch contracts and extend independent dispatch replay.
- Prior milestone: cddc2899c constructor/header 73/73 exact; live owner uncompared.
- Started merge of re-mysims-ms20; resolved additive CMake/config conflicts and removed obsolete open MS18/MS20 entries.
- Merged answered contracts in 16354d1ef; MS18 §12.29 supplies 21 prose vectors, not executable engine vectors.
- Added core route-ready ordinary/same-mode/FE/delay decisions, stuck ordering, deterministic model lookup, retry-false quirk and constructor defaults. Focused dispatch suite passed 33/33 (42 unequal pairs inside one test).
- Failed initial compile was private valid() visibility; fixed. q.py cannot launch Git Bash (CreateFileMapping error); direct cargo works.
- Next: complete idle/callback/route-partition inputs and executable conformance; extend plugin using only supported interface boundaries. MS20 explicitly forbids claiming live independent replay from snapshots; direct methods have no callable IID slots.
- Clean-room prohibition on running tools/ still applies; questions index regeneration delegated to coordinator (incoming index already has answered entries).
- No round-2 game request outstanding; unique labels required, wait via same exec session without qwait.
