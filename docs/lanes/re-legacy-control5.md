| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacy-control5 (legacy control behaviour spec) | `Codex` / `re-legacy-control5` (re, detached worktree) | ready; 12 interpreter contracts added, 57/57 owner functions inventoried with explicit edge limits; 33 spec vectors; doccheck and questions check passed; cargo/clippy/ctest n/a; game n/a; LC4 answered, LC11–LC13 filed; ABI unchanged | (1) Implement defined-input interpreter contracts; (2) resolve LC5, LC8–LC13 before claiming full widget/round-trip equivalence. |

## Report

The brief described an unspecified owner, but the reserved worktree already contained a 45/57-function
spec from re-legacycontrol4. This lane extended that document instead of replacing its prior work.

- Added sections 9.9–9.20 for the twelve outstanding interpreters: Text, TextEdit, ListBox, Combo, Btn,
  BMP, Scrollbar, LineInput, Spinner, TextTicker, SC3WinGen and OptGrp. The contracts record interface
  setter offsets, property typing, first/second-pass effects, defaults, exact-one/nonzero/inverted booleans,
  input-order population, resource metadata and failed-load effects.
- Retained the existing construction, vocabulary, owner inventory and three vtable tables. Corrected
  the UseWinProc slot description to the procedure field at object +0x18 and the Release section link.
- Added section 13's message/drawing boundary and failure matrix, with links to widget-toolkit,
  ui-windows and html-document. The adapter constructs/configures widgets; it has no message/paint slot.
- Added 33 spec-side conformance vectors LCV01–LCV33 in section 14. These are static-evidence predictions,
  not executable tests or game measurements. No engine, plugin, capi or coverage-map files were touched.
- Owner coverage is 57/57 contracts at the adapter-call level, explicitly excluding indeterminate scan/OOM
  outcomes and external helper/procedure internals. It is not a pixel, runtime or implementation verdict.

Evidence: export queries first, then bounded owner decompilation reads and image facts from the main
checkout. Instruction checks confirmed the unsigned alpha clamp to 255, binary32 multiplier 0x3B808081,
x87 multiply and binary32 argument store; signed Scrollbar limit ordering; 32-bit LineInput selection
subtraction; TextEdit's 15 property/mask pairs at 0x00AD56C8; and image/colour setter arguments obscured by
the decompiler. OptGrp's delimiter is a colon. REA was not needed; no shared REA session was queried.

Questions: LC4 moved to the answered archive with its rule in sections 9.9–9.20. Filed LC11 for unchecked
scan/allocation domains, LC12 for the external generator procedure and LC13 for the named-font helper
payload/indirect application. Existing LC5, LC8, LC9 and LC10 remain open. Files:
`docs/specs/questions/legacy-control.md` and `docs/specs/questions/answered/legacy-control.md`.
The question index was regenerated and checked.

Validation: `python tools/doccheck.py check`, `python tools/questions.py check` and `git diff --check`
passed after final spec edits. Cargo, clippy, ctest and game runs: n/a for this RE documentation lane.
No C ABI export changes, ABI row proposals, game requests or shadow comparisons.

Milestones: ef6cbe97 recorded scope; 62354d10 added interpreter contracts; bb5ae1d3 added failure/message
boundaries and vectors. Final report commit also clarifies the external font-helper limit and archives LC4
at the end of the answered file. Shared packed-refs lock warnings accompanied commits, but detached HEAD
advanced normally; no shared refs/config were changed.

## Handoff
- Goal: specify cGZLegacyControl in docs/specs/legacy-control.md without engine edits.
- Done: contracts, vectors and questions committed; final checks passed (see Report).
- Next: coordinator publishes detached HEAD to reserved lane branch and lands queued re documentation.
- Follow-up RE: LC5, LC8–LC13; engine implementers can consume defined-input effect tables now.
- No ABI changes or outstanding game labels; ignored evidence helpers remain in this worktree's build-runs/.
