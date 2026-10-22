# Answered tools COM director questions

## TD3 — Exceptions and singleton construction guard

`0094AC43` sets the static bit before constructor/exit registration. If construction
or AddCls throws, does the compiler unwind path reset the bit, release allocations,
or leave a partially constructed singleton?

**ANSWERED 2026-10-22 (re-tools-director4):** DIFFERS — §2 specifies retry after
exception: getter descriptor `00B03278` has one unwind state, mapping state 0 to
`00A6F206`, which clears bit 0 of `00B6351C`. Constructor descriptor `00B03254`
maps state 0 to `00A6F1F4`, which destroys its completed base. Both have no catch
table, so the exception propagates rather than becoming a false/null return.
Recursive entry without exception still sees the already-set flag and returns
the in-progress static storage. Exit registration's return value is ignored.
