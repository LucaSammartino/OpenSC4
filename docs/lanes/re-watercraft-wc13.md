| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc13 (WC10–WC12 behavior closure) | `re-watercraft-wc13` / RE lane, detached worktree | ready; prerequisite merge 0c46c082b; WC12 d978026d6; WC10/WC11 2cd367da3; doccheck passed, questions passed; 23 new spec vectors; no ABI change or game run | (1) Implement and test §§3.7–3.8; (2) apply corrected §5.7.5 contact sampling; (3) refresh owner README question references. |

## Report

Merged `core-watercraft-close12` first (0c46c082b) as requested. Its engine
changes are inherited prerequisite work; this RE lane authored only spec,
question-index and lane-report changes. No engine implementation files were
read; only the explicitly requested watercraft owner README was inspected.

Closed all three requested questions, without residual splits:

- WC10: §3.7 specifies resource type dispatch, sprite/mesh acquisition,
  untouched versus cleared failure outputs, reference ownership, auxiliary
  selector-to-property mapping and complete auxiliary insertion ordering.
- WC11: §3.8 specifies heading-basis arithmetic/fallback, inherited position,
  transform scale/revision/flags, first-model bounds, full/partial renderer
  submission, flag-dependent occupant publication and strict event threshold.
- WC12: p[24] governs both heel ramps and completion. Contact applies a fresh
  bank rotation to a **second terrain-normal sample**, not to -H or the incoming
  owner rotation. Corrected H09, retained H10, and documented zero-cross-product
  behavior as an unguarded quirk. Accepted publication still preserves the
  fetched owner scale.

Added 23 named conformance vectors: WC10-R01–R04/A01–A05,
WC11-P01–P04/U01–U07 and WC12-H11–H13. These are spec requirements, not claims
that engine tests were implemented or run. Updated §10's closure inventory.
Moved original question entries and dated answers to
`docs/specs/questions/answered/watercraft.md`; regenerated the index, which
reports watercraft 0 open / 12 answered / next WC13. No new questions filed.

Evidence came from main-checkout `tools/re_query.py`, bounded decomp reads and
instruction bytes for 0x007FEDE0, 0x007FD7D0, 0x007FED50, 0x0050EB20,
0x0050E4D0, 0x005FD5B0, 0x005094A0, 0x0080B830, 0x00508610 and the relevant
0x0056AF90 ranges. Numeric constants were checked in the executable.
No REA query was made: the session has no binary open, as the task states.

Owner README audit: **no row still cites an actually open watercraft question**.
The unchanged README has stale WC10/WC11/WC12 references in these seven rows:
§3 (WC10 acquisition), §3.5 (WC10 initialization), §3.6 (WC10 auxiliary work),
§5.1 (WC11 publication), §5.7.2 (WC12 endpoint), §5.7.5 (WC12 basis), and §10
(WC10–WC12 contract boundary). Its §6.2–§6.2.3 row also mentions WC9, already
answered before this lane. The introductory/final WC10–WC12 boundary prose is
likewise stale. Engine owners should refresh these references while preserving
their separately pending implementation/integration/test work; no README edit
was authorized here.

Validation: native foreground `python tools/doccheck.py check` passed, as did
`python tools/questions.py check` and whitespace checking. Gate output lives
under this worktree's `build-runs/`. The first wrapper attempt did not start
doccheck: Git Bash failed CreateFileMapping with Win32 error 5; the subsequent
native PowerShell gate passed. Git emitted packed-refs lock warnings while
committing, but the detached HEAD merge and all commits succeeded; no refs or
configuration were changed manually. Cargo/clippy/ctest and game/shadow runs:
n/a for authored spec-only work. ABI exports unchanged; no proposed ABI rows.

## Handoff
Goal complete: WC10–WC12 answered with behavior and conformance vectors.
Done: prerequisite 0c46c082b; WC12 d978026d6; WC10/WC11 2cd367da3; passing documentation gate.
Next: coordinator may land the queued detached HEAD; engine owner implements and validates the new contracts.
Facts: no pending game requests or open watercraft questions. README row references are enumerated in Report. No ABI changes.
