| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| cores20: network bridges core (network-bridges.md) | `cores20` / (engine lane) | **not merged**; branch `cores20` carries a merge of `cores17` (the network geometry core, which main still lacks) plus `engine/core/src/network_bridge/` (13 files: types, load, specialize, span, select, land, elev, model, pieces, cost, getters, mod, tests). Gate: cargo 2954/0, clippy clean, rustfmt clean on the touched files. 24 unit tests (`t17_NN_…`, the §17 tests) and 11 conformance vectors (`nbr17_01…nbr17_25`). No C ABI export added (`abi: false`); `FC4_ABI_VERSION` stays 69u. No plugin glue and no game runs (core-only lane). Questions 1-9 filed | Next: (1) land `cores17` first — `cores20` depends on `engine/core/src/network_geometry` and already merges it; (2) answer questions 2-5 (the `+0x0C` default, the duplicated `0x482CB448` property id, the `rev(...)` rebase, the per-cell low/surface measures); (3) implement §11.4's suspension profile and cable plotting once their formulas are in the spec (NB2); (4) add the plugin glue and a game run when a glue lane picks this up; (5) cover §17 tests 13, 15, 22, 23, 24, 26, 27, 29 with vectors (the unit tests and the code cover them, the vectors do not yet) |

## Next steps

1. **`cores17` must land first.** `network-bridges.md` §8.3 (NB1) and every model need
   `network_geometry`'s `Run`, `Step`, `StepKind`, `CellInfo`, `VertexHtConstraintSatisfier` and
   `GeometryTerrain`. That core is on branch `cores17` and is **not** in `main`'s tree
   (`git ls-tree main:engine/core/src/` has no `network_geometry`), although `main`'s `fc4.h`
   already carries the ABI-69 history line naming it. `cores20` merges `cores17` (commit
   `6ccf9a38`) so the types are available; if the train lands `cores17` first, that merge is a
   no-op.
2. The `GeometryHost` seam the geometry core already defines (`bridge_type_count`,
   `is_step_good_for_bridge`, `bridge_step_edge_types_ok`, `adjust_bridge_span`,
   `find_best_bridge_type`, `identify_land_bridge_cells`, `extra_network_constraints`) is the
   intended plug-in point; `network_bridge` implements the algorithms those hooks stand for and
   also exposes `identify_bridge_cells` (§4) end to end. A glue lane should wire the two.
3. §8.3.2 (fixed cells) and §8.3.4 (the plane constraint beside a bridge, network-geometry.md
   §9.8) are specified in the answer to NB1 but live in the geometry core's
   `extra_network_constraints` hook, which still has the empty default.
