| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| apply10b (EX/GF7/PR/LG/AO) | `a852c6ec244071bc2` / `agent-a852c6ec244071bc2` | Reserved ABI 57, seam op 56. **merged `0f3ad91`** (ABI 50, taken as 57; gate apply10b: cargo 1958/0, clippy clean, ctest 46/46; regress apply10b run 111 PASS): EX34, EX35, EX38, EX39 (slice setters always store an array; vector readers read a scalar as empty), GF7 readers (`fc4-dat models`, survey reproduced), PR17/PR18, LG21 (view rotation route; only rotation 0 seen), LG23, AO1; lane shadows exact (development 928/928, protection 288/288, landfill 72/72) | done. Questions EX40, GF15-GF18, LG30, AO3, PR20 filed. Open: `fc4-dat models` is not yet in engine/README.md or README.md; a run that rotates the view before zoning a landfill would check LG21 at a non-zero rotation (no script action rotates the view); the script action `zone 128 10 130 12 14` on Konradshohe crashes the game's own PlaceZone (`+0xDD90F`, also with the original classes). Optional: add landfill/protection replace runs like `apply10b-replace-2` to regress |

## Retired roster row

Kept verbatim from the HANDOFF §0 `| Lane | Agent ID | Worktree | Reserved | State |` table (2026-10-14 wind-down); the generator does not read this section.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| apply10b: exemplar EX34/EX35/EX38/EX39, GF7 readers, protection PR17/PR18, landfill LG21, agent order AO1 | `a852c6ec244071bc2` | worktree | ABI 57, seam op 56 | merged `0f3ad91` (ABI 50); EX40, GF15-GF18, LG30, AO3, PR20 |
