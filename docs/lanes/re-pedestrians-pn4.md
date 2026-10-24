| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pedestrians-pn4 (PN1 one-shot initialization and play count) | `Codex` / `re-pedestrians-pn4` (RE lane, detached HEAD) | in progress | (1) Merge core-pedestrians12; (2) trace PN1; (3) write behavior and conformance vectors; (4) doccheck and queue. |

## Handoff

- Goal: completely answer PN1 in pedestrians §2.1/§3/§7, including automaton slot 158 and the one-shot play-count consumer.
- Done: merged core-pedestrians12 at f60a2d43a; export search located one-shot Init at 0x00544DA0 and constructor at 0x00546860.
- Next: read pedestrians §2/§3 and query Init, the vtable target at slot 158, and the play-count consumer.
- Constraints: spec text only; no engine, plugin, capi, README, docs/abi, or coverage-map edits; no REA session is available.
- Evidence: main checkout C:/Users/tlori/FreeCity4; all scratch and gate logs under this worktree's build-runs.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-pedestrians-pn4-r1-manifest.json.
- Merge returned exit 0 and created f60a2d43a despite a packed-refs.lock permission diagnostic; detached HEAD verified by log. No manual engine reads or edits.
