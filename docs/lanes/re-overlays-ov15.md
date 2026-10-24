| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-ov15 — ordinary decal flags | Codex / re-overlays-ov15 | in progress | Trace bit 7 initialization and setter preservation; close OV15 and run doccheck. |

## Handoff

- Goal: answer OV15 from glue-overlays-live6, with behavior and conformance cases only.
- Spec milestone: §3.2.2 now proves complete words 91/91/91/91/91/D1 and unconditional creation-time bit 7; §9.3 adds four paper vectors.
- OV15 copied verbatim from sibling branch to answered/overlay-manager.md; question index regenerated.
- Export instruction proof: default flags 1 at 0x007360F1, binding OR 90 at 0x0073C011, source OR 90 at 0x0073B0CF, self-lighting OR 40 at 0x0073656F.
- Owner closure table was not found in the sibling lane report or overlay spec. Engine reads are forbidden; no engine README opened.
- q.py cannot start Bash in this sandbox (CreateFileMapping error); native PowerShell commands work.
- No engine files read or changed; no ABI changes or game runs requested.
- Next: review documentation diff, run doccheck, commit final report and enqueue actual detached HEAD.
