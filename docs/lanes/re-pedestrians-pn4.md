| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pedestrians-pn4 (PN1 one-shot initialization and play count) | `Codex` / `re-pedestrians-pn4` (RE lane, detached HEAD) | in progress | (1) Merge core-pedestrians12; (2) trace PN1; (3) write behavior and conformance vectors; (4) doccheck and queue. |

## Handoff

- Goal: completely answer PN1 in pedestrians §2.1/§3/§7, including automaton slot 158 and the one-shot play-count consumer.
- Done: prerequisite merge f60a2d43a; initial lane record ec738e0d7; PN1 behavior and 22 vectors written in pedestrians §2.1/§3/§8; question moved to answered/pedestrians.md.
- Next: regenerate questions index, run questions check and doccheck in the foreground, inspect only log tails/failures, then commit this milestone and report.
- Settled: slot 158 is 0x00544620; one-shot slot 72 is 0x00544E80 (entry 0x00A9AC70); direct byte count +0x270; first non-null model only; final-frame samples decrement even with zero delta; Remove(false) at zero returns false but later models still advance.
- Settled: tick forwards its second argument (binary32 milliseconds) to opacity slot 83 and model Advance; first microsecond argument unused. Init success always sets model speed 0x3C23D70A and redraws count, including repeated Init.
- Evidence: decomp/_unnamed/region_00540000.c lines 752–798 has tick pseudocode; instructions correct its wrong field index/undefined result. Main decomp one-shot Init failed; export instructions supplied the complete body, without REA.
- Constraints: spec text only; no engine, plugin, capi, README, docs/abi, or coverage-map edits; no REA session is available.
- Evidence: main checkout C:/Users/tlori/FreeCity4; all scratch and gate logs under this worktree's build-runs.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pedestrians-pn4-r1-manifest.json.
- Merge returned exit 0 and created f60a2d43a despite a packed-refs.lock permission diagnostic; detached HEAD verified by log. No manual engine reads or edits.
