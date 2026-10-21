| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budget-arrow4: answer budget-window BW12 (selector-specific initial arrow bounds, RE spec lane) | fc4-lane agent, .claude/worktrees/re-budget-arrow4 | ready: BW12 answered DIFFERS (ordinance helper hides down arrows at count < 9, neighbour helper at <= 9 like group mode) | core lane: implement the per-selector bounds in spec 5.3 and test 18; game lane: BW11 (still open, not touched) |

## Report

What changed (docs only):
- docs/specs/budget-window.md section 5.3: new paragraph "Selector-specific initial arrow state" (BW12) with a
  per-helper bound table (group / ordinance / neighbour), where each helper's counts come from, and worked boundary
  cases for counts 0, 8, 9, 10 and 0x80000000. Section 9: conformance test 18.
- BW12 moved to docs/specs/questions/answered/budget-window.md (DIFFERS); QUESTIONS.md regenerated.

Measured facts (static, from re_query.py asm --bytes; REA not used):
- initOrdinanceBudget 0x0077C660 (caller on_init 0x00787C42): up arrows hidden always; `CMP [+0x124],9` at 0x0077D73E
  then `JAE` (73 1D) skips the 0x452 hide; `CMP [+0x128],9` at 0x0077D77C then `JAE` skips the 0x454 hide. Hidden
  when count <= 8. Counts are only incremented by the helper (0x0077C734/0x0077C73C) and start at the constructor's
  zeros (0x007864AE/0x007864B4).
- initNeighborBudget 0x0077E600 (caller 0x00787D5D): same layout, `JA` (77 1D) at 0x00781BEC and 0x00781C2E: hidden
  when count <= 9, same as group mode (0x007896D6 `JA`). SetFlag arguments come from EBX = 0 (XOR at 0x00781887).
  Counts reset to 0 at 0x0077E6A6/0x0077E6AC and include a header row per section.
- All compares are unsigned u32.

Questions: BW12 answered; none filed. BW11 (runtime Alt delivery) left open.
