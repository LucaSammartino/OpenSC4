| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-budget-core-final4: BW13 (on_init `+0x10` create failure) and BW14 (`+0x38` department refcount) answered from native bytes | Claude / .claude/worktrees/re-budget-core-final4 | ready: spec §4 + tests 19-20, BW13 MATCHES (control flow; native then faults), BW14 DIFFERS | core: count department refs in `+0x38` (AddRef on save, Release on clear/shutdown/drop); no-fault path for missing `+0x10` |

## Report

**What changed** (docs only; no engine/tool/ABI edits):
- docs/specs/budget-window.md §4: "Failed content creation" (BW13) and "Department references in the saved pairs"
  (BW14, refcount table); §9 tests 19 and 20.
- docs/specs/questions: BW13, BW14 moved to answered/budget-window.md; QUESTIONS.md regenerated.

**BW13 (MATCHES on control flow, native crash noted).** `CreateInstance` false → `JZ` 0x00787AD2 to 0x00787BDA:
guarded Release of the generator out-pointer, then the selector branch at 0x00787BE7 as on success. Pause
(0x007878C0) stays held, `+0x08 := 1`, return 1. No null check of `+0x10`; every mode builder passes it to
`add_text` (0x00779660), which dereferences the parent at 0x0077982B, so the native faults. The spec marks the
no-fault continuation (child insertion into missing `+0x10` = no-op) as a *[divergence]*.

**BW14 (DIFFERS from core's uncounted handle).** Per department: temp AddRef 0x00787FF6, element AddRef on append
0x00788023 (grow path FUN_0077D7E0 copies with AddRef via FUN_0077AC20/FUN_0077AC60 and Releases old elements
via 0x005FF0C0), temp Release 0x007881BD; net +1 per pair. Released per element by erase routine 0x0077C420
(on_init 0x00787F8B, on_shutdown 0x0077E46F) and by the destructor 0x0077E420 → 0x0077C4C0 → 0x005FF0C0
(0x0077C592). `+0x2C` slider rows use the same release routines; their append was not byte-checked.

**Method:** static only (re_query.py asm; REA not used). BW11 (runtime Alt delivery) untouched, no runtime claim.

**Next:** core lane implements test 19/20 behaviour; optionally byte-check the `+0x2C` SliderInfo append AddRef.
