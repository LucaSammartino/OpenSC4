| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-construction: the construction occupant's lot division, cranes, prop tables and cleanup, folded into lot-developer.md §6.7 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4abc0e4`, after re-fonts) | **ready** (docs and RE tables only). lot-developer.md §6.7.10 (the record points at lot-records.md §9.1; who writes the world box), §6.7.11 (`DivideLot`), §6.7.12 (crane size tables, outer-crane fit and position, inner crane, `RemoveRectangle`, `SetCraneAngle`), §6.7.13 (prop tables), §6.7.14 (`Shutdown` cleanup) and §6.7.15 (what stays open) replace the "Open" subsection; §6.7.1 and §6.7.7 point at them; tests 44–47; lot-records.md §9.1 row 6 completed; OP17 answered; 19 names (1 new). No coverage change (`cSC4ConstructionOccupant` was already claimed) | the model geometry and the inner crane's placement (§6.7.15) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-20 as new §6.7.10–§6.7.15 of lot-developer.md and held back when
`main` rewrote §6.7 and lot-records.md §9 (core-construction lanes: the three `this` frames, every record field, the
swing limits); this fold keeps upstream's text and adds what its §6.7.10 still listed as open.

### What changed

- lot-developer.md §6.7.10: the field table of the held lane is dropped (lot-records.md §9.1 has it); what stays is
  OP17's answer: `RebuildModel` writes the world box (`0x005CB76D`–`0x005CB799`) as the scaffold box widened by
  sfPostWidth in x and z.
- §6.7.11 `DivideLot` (the eight areas), §6.7.12 the crane size tables (measured from the three crane models), the
  outer crane's preconditions, usable-side tests and position, the inner crane's size rule (placement in summary),
  `RemoveRectangle`, `SetCraneAngle` and the smoothstep swing (the swing range itself is upstream's §6.7.6), §6.7.13
  the ground and roof prop tables, §6.7.14 `Shutdown`'s cleanup (with the quirk that the current effect is released
  without a null test), §6.7.15 what stays open.
- §6.7.1 and §6.7.7: `Shutdown` removes everything the occupant placed, which settles the inferred note on the
  completion's props and crew.
- Tests 44–47; lot-records.md §9.1 row 6 says who writes the box.

### Names and coverage

- re/symbols/construction-occupant.tsv: 19 names (1 new, `SetCraneAngle` `0x005CBE10`; 18 confirm auto names). The
  `names_windows.tsv` delta (−18 +19) is identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- tools/coverage-map.tsv: the `cSC4ConstructionOccupant` row cites lot-records.md and lot-developer.md. Specified
  bytes unchanged (3,478,316).

### Questions

- Answered: OP17 (the world box).
- Nothing new filed; §6.7.15 lists the remainder.

### Not covered

The model geometry (`RebuildModel`'s mesh, `SetScaffoldHeight`, `CreateCrane`, `Draw`, `DoRotationChange`), the inner
crane's exact placement, the render setting's name.
