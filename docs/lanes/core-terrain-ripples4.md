| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-terrain-ripples4: terrain ripples and selection | Codex / `core-terrain-ripples4` (engine lane) | in progress | 1. Implement bounded public spec; record missing contracts. 2. Focused checks, final full gate. |

## Handoff
- Goal: pure Rust cSTEWaterRipple and cSTESelectedTerrainArea from water-ripples §§1–2,6; no ABI/plugin/runtime claims.
- Worktree reused, detached HEAD; initial git status clean, no main.. commits.
- Baseline: coordinator states stable native train gate passed; task explicitly forbids repeating pre-edit full baseline.
- Read: HANDOFF index, contributing rules, lane/queue formats, water-ripples §§1–2,6, questions WR1–WR3.
- Next: inspect public SDK signatures and core patterns; implement literal contracts and file precise WR4+ gaps.
- Timing stays in raw units (175000 / 2000000), flags/selection bytes opaque.
- No game request needed or outstanding; core-only lane, no shadow or runtime credit.
