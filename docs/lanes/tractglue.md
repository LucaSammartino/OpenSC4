| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tractglue (tract developer) | `ae81a8bc4c10d6880` / `agent-ae81a8bc4c10d6880` | Reserved ABI 45, seam op 45. **merged `76bf685`** (gate tract-final: cargo 1946/0, ctest 46/46; regress `mtract` run 109 PASS): shadow exact (`tract-t6s`: 4092 calls compared, 0 mismatches; UpdatePopulation 601524 compared, 0), ours clean at normal speed to 2038 (`tract-t5r`); cargo 1946/0, clippy clean, ctest 46/46; ABI 48, seam op 45, 196 switches | done. Next: add tract to the regress replace and shadow lists. Optional: requeue `tract-t6t` (turbo replace) and `tract-t6v` (save) from its `done/` requests. 19 questions in its report (Grow is 1.5-2.5x the original; a pooled allocator for the staticlib would help all subsystems) |

## Retired roster row

Kept verbatim from the HANDOFF §0 `| Lane | Agent ID | Worktree | Reserved | State |` table (2026-10-14 wind-down); the generator does not read this section.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| tract developer glue (M4) | `ae81a8bc4c10d6880` | `agent-ae81a8bc4c10d6880` | ABI 45, seam op 45 | running |
