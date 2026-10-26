| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-layoutview8: the network layout view (the network/zone overlay) and its GZCOM custom view with the ferry-route path view, network-layout-view.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-nettools8) | **ready** (docs only). Three owners claimed: specified +7,858 bytes (+14 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly; the type order, propagation, spreading and triangle tables, the default
palette, the zone jump table and the smoothing constants were read in the PE. Written on 2026-10-25.

### What changed

- docs/specs/network-layout-view.md (new): §1 identities, layouts and the 14-colour palette; §2 Init (render state,
  the palette exemplar) and Update (batched triangle draw); §3 RebuildMap: the per-side code grid from every network
  occupant, the two spreading passes of the wide networks, the zone and network triangles with their heights; §4 the
  custom view: creating the layout view and the ferry-route path view, the two Bézier-midpoint smoothing passes,
  shutdown; §5 quirks; §6 conformance tests.

### Coverage

- tools/coverage-map.tsv: rows for `cSC4NetworkLayoutView`, `cSC4NetworkLayoutCustomView` and its class-id function,
  before the `cSC4NetworkLayout*View` row.
- Specified bytes 5,093,310 → 5,101,168 (+7,858, 91.4%, `cov_local.py` with owners-c1).
