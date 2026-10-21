| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-construction-contractfix4 (apply OP13-OP16 answers to construction occupant, lr §9.1/9.2, ld §6.7.1/6.7.6/6.7.7) | `core-construction-contractfix4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: re-key ConstructionScalars to lr §9.1 object-frame roles; resume schedule from #14; refs #3/#4 live vs #8 load-only (+D8/+D9 refresh); Init = base Init + 4 subs + AddOccupantGroup(0x5007); sparks x/z extent; crane T/k/swing (ld §6.7.6). Keep v4 transform.
- Done: c4cc91d7 implementation + tests (quick PASS, fc4-core 2257/0); OP17 filed (world box #6 writer).
- Next: qbuild all; then final report + merge-queue json. game=false (core only, no plugin wiring).
- Facts: one round only; park if unfinished.
