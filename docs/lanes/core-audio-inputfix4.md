| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-inputfix4: conform ambience owner to audio-ambience AA5-AA9 answers, tests 19-24 | `core-audio-inputfix4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: apply AA5-AA9 (alarm last=hour, margin/id vectors, prime-step iterator, move/automaton instance ids, flag 1 starts, focus stage 1/4, master vol 0, alarms kept at Shutdown) to engine/core/src/audio/ambience.rs; conformance 19-24; full gate.
- Done: worktree created (base 802fa8d4).
- Next: read spec §2-§7, §10; baseline gate `qbuild all`.
