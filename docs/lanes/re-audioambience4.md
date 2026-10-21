| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audioambience4 (RE spec: audio ambience AA2/AA1) | `re-audioambience4` / (RE lane) | in progress | AA2 then AA1 |

## Handoff
- Goal: answer AA2 (0x004E9DA0 condition change) then AA1 (ambience layer 0x004E5110-0x004E8230) in docs/specs/audio-ambience.md; AA3/AA4 if cheap.
- Done: AA2 answered in spec 3.4 (commit 1).
- Next: AA1 layer internals, decomp/cSC4AudioAmbienceLayer.c (3035 lines), REA unavailable (no app open).
