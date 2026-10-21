| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-budgetwindow-final4: budget-window.md §4/§4.0/§5.3 BW10/BW12 in core budget_window, §9 tests 14-18 | fc4-lane agent, .claude/worktrees/core-budgetwindow-final4 | ready: core only, gate PASS, no game run (core-only lane, no plugin/CAPI edits); BW13/BW14 filed, BW11 runtime-only | answers to BW13/BW14; glue lane implementing the new host methods |

## Report

### What changed (engine/core/src/budget_window/ only)
- Content window `+0x10` (BW10a): `on_init` creates it after the pause and before the selector branch, in every mode: `create_content_window`, AddRef (the auto-ref), 8 generator flags, 6 window flags, `SetID(0x0423278E)`, `ChildAdd` on the dialog, then the temporary generator reference is released. `on_shutdown` releases `+0x0C` then `+0x10`; `release_final` releases `+0x1C`, `+0x14`, `+0x10`, `+0x0C`. `content()` getter.
- Empty group (BW10b): `+0x38` is emptied, then filled inside the counting loop with one pair per department before any visibility test. An all-hidden group keeps its pairs but has no slider rows (`+0x2C`) and no arrows.
- Initial arrows (BW10c/BW12): `create_scroll_arrows` runs four `add_image_button2` calls (up `0x140155CB`, down `0x140155CC`), each followed by its notification target. Both up arrows are always hidden. A down arrow is hidden when its count is <= 9 in group and neighbour mode and < 9 in ordinance mode; otherwise it is left as created. Counts are unsigned u32. Ordinance counts only add the helper's `added_a`/`added_b`; the neighbour helper's `NeighborInit` counts replace them.
- Host trait: new `create_content_window`, `child_add`, `add_image_button2`, `set_notification_target`. `init_neighbor_budget` now returns `NeighborInit`, and `OrdinanceInit` has two new fields, `added_a` and `added_b`. Only the test mock implements the trait.
- Unchanged: the unguarded HiddenResume on create failure; the raw Alt bit-0x4 test (the comment now says BW5 is answered and BW11 is runtime-only).

### Measured
- Baseline qbuild all: 3980 tests passed, clippy 0. Its ctest showed 1 failure because I edited files during the run.
- Final qbuild all: 3986 passed / 0 failed, clippy 0, ctest 65/65, qbuild PASS.
- rustfmt --check: clean on the touched files.
- New tests: alt_enter_and_alt_f4_close (§9 test 14), content_window_created_in_every_mode (15), release_final_releases_content, empty_group_keeps_saved_pairs (16), group_initial_arrows (17), selector_initial_arrows (18 plus the boundary table).

### Questions (docs/specs/questions/budget-window.md)
- BW13: does a content-window create failure skip only its block (implemented) or the whole mode branch?
- BW14: is the department in a `+0x38` pair AddRef'd, and where is it released? The core makes no call.

### Not covered (spec out of scope, §8)
- Populate-routine layout and string loads.
- The `+0x18` field: nothing in the spec sets it.
- BW11: Alt+Enter / Alt+F4 delivery at runtime.

### Next
- A glue lane implements the four new host methods and the `NeighborInit`/`OrdinanceInit` counts in the plugin.
- Apply BW13/BW14 once they are answered.
