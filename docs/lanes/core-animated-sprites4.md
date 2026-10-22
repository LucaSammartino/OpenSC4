| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animated-sprites4: animated sprite owner | Codex / `core-animated-sprites4` (engine lane) | in progress | 1. Read public contracts; implement complete specified owner behavior or file precise contract holes. |

## Handoff
- Goal: pure Rust animated sprite instance owner, core only; no resource-owner or model_instances edits.
- Reserved worktree reused; detached HEAD; no pre-edit baseline rerun (fresh coordinator baseline supplied).
- Done: initial metadata b7843313; all requested spec sections and narrow public base/resource/transform contracts read.
- Done: d3c758b1 WIP owner components, 29 unit tests, inventory in engine/core/src/animated_sprite/README.md.
- Done: focused quick PASS, core 2439/0; dependent crates compile-only; CMake 206/206 edges.
- Filed AS3-AS9 in docs/specs/questions/animated-sprite.md; existing AS1/AS2 remain open.
- Done: six new Rust files rustfmt check PASS; lib.rs check finds pre-existing module-order/format drift (no mass reformat).
- Next: ONE stable final all gate; final report/queue must be WIP (incomplete native contracts).
- Native 60-function owner inventory/defaults, table stepping, sentinel rates, orientation, full draw/pick/bounds/sort are absent from spec.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-animated-sprites4-r1-manifest.json.
- No game request outstanding; core-only work has no shadow or ABI surface.
