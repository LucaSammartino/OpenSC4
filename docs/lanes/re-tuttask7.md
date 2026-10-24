| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tuttask7: the tutorial step `cSC4TutorialTask`, new spec tutorial-task.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-gameprefs7) | **ready** (docs only). tutorial-task.md §1–§7: fields and the 27-slot interface, the Lua reading of a task (messages, numbers, `task_action`, `target_buttons`), the cell grid of types 2 and 0x11, `GetTargetWindow`, start/end/highlight/fire and the camera move, quirks, five conformance cases; TUW5 of tutorial-window.md answered; owner claimed: `cSC4TutorialTask`: specified +6,340 bytes (+33 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 33 claimed functions were disassembled. The 21 Lua key names were read from their start-up string initializers
(`0x00B21048`–`0x00B211EC`), the start jump table from `0x00440730`/`0x00440740`, the vtable from `0x00A862D8`.
Written on 2026-10-23. tutorial.md only outlined tasks; the upstream tutorial core does not cover this owner.

### What changed

- New docs/specs/tutorial-task.md.
- Read on the bytes:
  - the Lua keys and their fields;
  - the number reader that disables later reads on a missing key;
  - the text-database loading of the three messages with the `***_NOT_IN_TEXT_DATABASE_***` fallback;
  - the button list (hex strings, target GUID first for type 4);
  - the cell grid with end flags for type 2;
  - the hint window walk (visibility, the `0x0A78827A` → `0xC991EDA8` substitution before the city is established);
  - the per-type start actions (highlight effect, hidden pause, fire);
  - the CSI visibility save and restore;
  - the camera move to the cell centre.
- tutorial-window.md: TUW5 answered (§8 table and pointer updated; the entry moved to
  questions/answered/tutorial-window.md). QUESTIONS.md regenerated.

### Quirks found

- A missing number key disables the rest of the task's reads.
- Index 0 reads from `tutorialtasks` itself.
- `End` of a type-0 task returns false.
- A camera target on cell 0 is skipped.
- Every listed button that exists is enabled.

### Coverage

- tools/coverage-map.tsv: the `cSC4TutorialTask` row now cites tutorial-task.md.
- Specified bytes 4,542,360 → 4,548,700 (+6,340, 81.3% → 81.5%, `cov_local.py`, on `5c0ff782` + the eighteen lanes
  before).

### Not covered

- The readers of the cell grid (tutorial system); the `cADVString` loader.
