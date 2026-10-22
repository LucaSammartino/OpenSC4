| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-terrain-ripples4: terrain ripples and selection | Codex / `core-terrain-ripples4` (engine lane) | in progress | 1. Implement bounded public spec; record missing contracts. 2. Focused checks, final full gate. |

## Handoff
- Goal: pure Rust cSTEWaterRipple and cSTESelectedTerrainArea from water-ripples §§1–2,6; no ABI/plugin/runtime claims.
- Worktree reused, detached HEAD; initial git status clean, no main.. commits.
- Baseline: coordinator states stable native train gate passed; task explicitly forbids repeating pre-edit full baseline.
- Read: HANDOFF index, contributing rules, lane/queue formats, water-ripples §§1–2,6, questions WR1–WR3.
- Implemented: tuning, flags, spline/normal/pair/texture/animation kernels, delayed rebuild state, five selection marking forms and Arc lifetimes; 37 independent Rust tests.
- Questions filed: WR4–WR10, covering trace, full strip traversal/preparation, overshoot, drawing/masks, native lifecycle/inventory, selection data gaps, edit boundaries.
- Quick gate PASS: core 2447/0, dependent crates compile, native build 206/206; six new Rust files fmt PASS.
- Full owner checklist is public in terrain_ripples/mod.rs. Neither native owner complete; no invented host trace/mask callbacks.
- Next: stable final qbuild all, then report/commit/queue as wip with explicit spec blockers.
- Timing stays in raw units (175000 / 2000000), flags/selection bytes opaque.
- No game request needed or outstanding; core-only lane, no shadow or runtime credit.
