| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budget-slider-ownership4: BW15 `+0x2C` SliderInfo department refcount from native bytes | Claude / .claude/worktrees/re-budget-slider-ownership4 | ready: spec §4 BW15 block + tests 21-25; BW16 filed (§4.1 ID-test direction) | core: count department refs in `+0x2C` (one per element), erase order `+0x2C` then `+0x38` on shutdown, reverse in dtor; review BW16 |

## Report

Static only (re_query.py asm --bytes on the export; REA not used).

What changed (docs/specs/budget-window.md):
- §4, after the BW14 block: "Slider rows `+0x2C`: what a `SliderInfo` owns" (BW15). Element = {u32 slider ID, counted
  department}. No reference to the slider window: `FUN_007794E0` makes it, ChildAdds it to the dialog `win` (first
  argument, not `+0x10`) and Releases its creation ref (`0x0077958C`). Two append sites (`0x00788D6E` B header row,
  `0x007891BD` B item row), both via push_back `FUN_0077E550` and keyed to the outer department. Per element: temp
  AddRef +1, push_back AddRef +1 (fast `0x0077E573`, grow `FUN_0077D7E0` net +1), temp Release -1, so net 1 ref per
  element. Erase/clear/destroy order table: on_init clears `+0x2C` only in the non-empty branch (`0x00788376`), after
  `+0x38` is filled; on_shutdown erases `+0x2C` then `+0x38`; dtor releases `+0x38` then `+0x2C`; each front to back.
  BW13 interaction: native faults at `0x0078837B` before any slider push; under the no-fault rule sliders are built
  normally.
- §9 tests 21-25: empty groups, non-empty with 9-row cap (counts X+1, Y+4, Z+7), repeated on_init, shutdown then drop
  with Release order, failed content creation.
- §8 mentions BW16.

Questions: BW16 filed in docs/specs/questions/budget-window.md: §4.1 states "shown if ID > 100 or amount != 0", but
all four sites (`0x007880D3`, `0x00788147`, `0x007886ED`, `0x00788F33`) are `CMP ID,0x64; JBE shown`, then a zero
amount hides the item, so ID <= 100 is always shown. BW15 tests avoid this case.

Commits: 6a4af384 (BW15 spec), f0e15d0a (tests, BW16), then this report.
