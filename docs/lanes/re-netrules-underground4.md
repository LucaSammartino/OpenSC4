| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netrules-underground4: NR15 underground piece application | Codex / `re-netrules-underground4` (RE spec) | blocked on Git administrative write permission; proof complete but uncommitted; NR15 answered DIFFERS; questions check and diff check PASS; gate/game n/a; ABI unchanged | 1. Coordinator commit the preserved docs. 2. Mark queue ready after commit. 3. Use the native underground slot-30 contract in glue. |

## Report

NR15 is answered **DIFFERS** in network-rules.md §11.3 and the answered question archive. The open entry was
moved, and the question index regenerated. Sections 11/11.2 now distinguish the surface implementation from
underground dispatch. Tests 32–35 supply native-derived discriminating vectors; these are specifications, not
claims of executed conformance or game verification.

Native results, from main-checkout export instruction bytes and read-only executable table words:
- Pipe tool vtable `0x00AA99A8` dispatches slot 54 through `0x00657740` to slot 82 `0x00648D10`;
  class `cSC4PipeOccupant`, class id `0x49C05B9F`, primary vtable `0x00AA97B8`.
- Subway tool vtable `0x00AAA5C0` dispatches the same adapter to slot 82 `0x00653A00`;
  class `cSC4SubwayOccupant`, class id `0xCA16374F`, primary vtable `0x00AAA3C8`.
- Both occupants' slot 30 is `0x0065ADB0`: thiscall, u32 id and four-byte orientation stack slot, only the low
  orientation byte consumed, eight-byte callee cleanup. It calls the underground height helper `0x0065AB50`;
  it lacks the surface setter's flag, path-cache, texture-coordinate and alpha work.
- Both tools use common slot 72 `0x006278A0`. Fresh no-exemplar commits apply at `0x0062797F` before
  `InsertOccupant` at `0x00627A38`, via the derived factory and setter, not an ordinary surface class.
- Underground slot 55 `0x006599D0` first calls the base, then runs a second pass under the documented control
  bytes. The absent branch applies at `0x00659747` before fallback insertion at `0x00659ABC`. Found occupants
  receive height/edge/flag updates without another SetPiece or insertion. Scalar widths, flags, argument-pointer
  adjustments and returned/cell/local reference ownership are specified with native address citations.

Validation: `python tools/questions.py index` and `check` PASS; `git diff --check` PASS. No engine files read or
edited; no code, tools or ABI changes; no game runs; cargo/clippy/ctest n/a for documentation. No REA was needed.
No runtime replacement or coverage credit is claimed. ABI proposal file: none.

Commit blocker: ordinary `git add` and `git commit -F` attempts failed with `Permission denied` creating
`C:/Users/tlori/FreeCity4/.git/worktrees/re-netrules-underground4/index.lock`. A read-only ACL check showed an
explicit deny Write on that directory. No permission bypass was attempted. The original tip is `befef312`;
all proof remains in working-tree docs, and this lane report is untracked until the coordinator stages it.
Commit message files are preserved in `build-runs/msg-re-netrules-underground4-1.txt` and `-2.txt`.
The queue entry is **wip**, explicitly recording that its tip does not contain these edits. This single scoped RE
round is complete; no further RE restart is requested. The coordinator can commit the existing files, then queue
that committed tip as ready. The implementation follow-up must use underground dispatch and preserve the found
branch behavior; the surface-only target check cannot simply be widened to call the surface routine.


### Coordinator delivery recovery (2026-10-22)
After the worker stopped on sandbox-denied Git writes, the coordinator reviewed and committed these exact docs files. The original guard result and proof evidence remain preserved; no extra lane round or runtime result is claimed. The queue records the real committed tip.
