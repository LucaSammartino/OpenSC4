| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-simmisc7: small simulation and region owners (the regional flat terrain, terrain and edge-reconcile dialogs, rule text helpers, lot/network/traffic record copies, render helpers), sim-misc.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-widgetmisc7b) | **ready** (docs only). 21 owners claimed: specified +8,522 bytes (+94 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 94 functions were read in the decompile; the regional terrain map's table and cell conversion and the dialog
bodies in the disassembly. Where another spec already describes an owner's main body (region-model.md,
foundations.md §11.6, network-rules.md, lot-developer.md, traffic-simulator.md, advisors.md, lot-configuration.md)
the row names both. Left out: `cSC4Framework`, `cSC4MessageServer`, `cSC4MessageServer2` (framework overrides,
gz-framework-core area), `nsTexUtil` and `SC43DGridElement` (renderer, not read to the end here). Written on
2026-10-24.

### What changed

- New docs/specs/sim-misc.md.

### Quirks found

- The region's stand-in terrain answers every height query with 270 m and every normal with straight up.

### Coverage

- tools/coverage-map.tsv: rows before `cSC4Region*`, `cSTEEdgeReconcile*`, `cS3D*` and `cSC4*`; six rows of the
  attributed-owner block filled in place.
- Specified bytes 4,941,806 → 4,950,328 (+8,522, 88.7%, `cov_local.py`).
