| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tutwin7: the tutorial window `cSC4TutorialUIWinProc`, new spec tutorial-window.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-ticker7) | **ready** (docs only). tutorial-window.md §1–§9: identity and fields, `Init`, the tasks (Lua loading, text, restart, step, reset, events, the end), the HTML comment commands, the UI hints (the two window walkers, the hint and catalog handlers, the self message), the window procedure and the exit dialog, the arrows and `Shutdown`, quirks, six conformance cases; questions TUW1–TUW6; owner claimed: `cSC4TutorialUIWinProc`: specified +7,407 bytes (+19 functions) | TUW1–TUW6 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 19 claimed functions were disassembled. The two static strings (`"tutorialtasks"`, `"sc4://tutorial"`) were read
from their start-up initializers (`0x00A71530`, `0x00A71B30`), and the vtables from the data section. Written on
2026-10-23. tutorial.md (§4.2.2, §4.4.1) already gave the owner-facing boundary of this window; the new spec builds
on it and does not edit it. No upstream lane covers this owner.

### What changed

- New docs/specs/tutorial-window.md.
- Read on the bytes:
  - the layout (script window, HTML control, arrows, exit dialog, highlight, rest position, task vector);
  - `Init` (script key `0x0A2DD355`, HTML control `0xAA12E5F5` in child `0x0A35F767`, the arrows);
  - the Lua task table `tutorialtasks` and the task build;
  - restart, step and reset, including the window placement, zoom, simulator resume and clock calls;
  - the `<!-- legacy … -->` commands (`id`, `winflag_visible`, `winflag_enabled`, `href`);
  - the two window walkers and their 2-visit cap per window ID;
  - the hint handler (task types 0xB–0xD, 0x10, others) and the catalog handler;
  - the link, attach and button messages, and the three exit-dialog buttons.
- docs/specs/questions/tutorial-window.md: TUW1–TUW6 (new prefix TUW); QUESTIONS.md regenerated.

### Quirks found

- The comment scan never ends when a `<!--` follows the text's first `-->`.
- The comment length is the `-->` index minus 1, not the distance from `<!--`.
- `href` values are lowercased.
- `finish_tutorial` ends the current task twice.
- `+0x3B` (last task) is only updated by `moveToNext`.
- The 2-visit cap leaves deep descendants of allowed buttons enabled.

### Coverage

- tools/coverage-map.tsv: `cSC4TutorialUIWinProc` → tutorial-window.md, before the `cSC4*WinProc` catch-all.
- Specified bytes 4,508,750 → 4,516,157 (+7,407, 80.7% → 80.9%, `cov_local.py`, on `5c0ff782` + the eleven lanes
  before).

### Not covered

- The HTML control (out of scope) and `cSC4TutorialTask` (TUW5).
- The message target's `DoMessage` and the click filter (owner `cSC4TutorialSystem`).
