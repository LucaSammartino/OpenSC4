| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mmutil8: the `cSC4ModelMakerUtility` helpers graphics-formats.md §4.2 leaves out, model-maker-utility.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-mapstreet8) | **ready** (docs only). One owner claimed with graphics-formats.md: specified +2,909 bytes (+11 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly. Written on 2026-10-25.

### What changed

- docs/specs/model-maker-utility.md (new): §1 the instance-id arithmetic, the occupant overload of
  `GetModelInstanceID` and a pointer helper; §2 the building-style model tests; §3
  `GetOccupantOrientationAndState` by occupant interface; §4 `GetOccupantModelClass` and `HasOccupantModelClass`
  (the state pairs and the quirk that the second ignores the state); §5 conformance tests. With graphics-formats.md
  §4.2 the owner is complete.

### Coverage

- tools/coverage-map.tsv: a `cSC4ModelMakerUtility` row before the `cSC4Model*` catch-all.
- Specified bytes 5,118,978 → 5,121,887 (+2,909, 91.7%, `cov_local.py` with owners-c1).
