| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-simmisc7b: 38 small simulation owners (iterators, occupant and lot filters, records and small constructors, scoped helpers), sim-misc2.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-simmisc7) | **ready** (docs only). 38 owners claimed: specified +3,574 bytes (+52 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 52 functions were read in the decompile. Most are named by the specs that use them (zones.md, lot-developer.md,
network-manager.md, occupant-manager.md, city-tools-input.md, tract-developer.md, automata.md, traffic-simulator.md,
prop-occupant.md, regional-city-records.md, tutorial.md, terrain.md); the rows name those specs where they describe
the body. Written on 2026-10-24.

### What changed

- New docs/specs/sim-misc2.md.

### Coverage

- tools/coverage-map.tsv: rows before `fn:SC4*Region*`, `cS3D*`, `cSC4*` and `cSC*`; 14 rows of the attributed-owner
  block filled in place.
- Specified bytes 4,950,328 → 4,953,902 (+3,574, 88.7%, `cov_local.py`).
