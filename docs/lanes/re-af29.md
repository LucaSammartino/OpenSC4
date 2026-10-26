| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-af29 aircraft AF2–AF6/AF10/AF11; kind docs | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/re-af29; detached HEAD, reserved branch re-af29; content commits 8479449bf, ce6fd7f10, 161ad079e, 651c30d00 | ready; doccheck passed, questions check passed, diff check passed; cargo/clippy/ctest n/a; game/shadow n/a; ABI unchanged, no seam/ABI reservations; seven questions archived, AF10 specified and six runtime-narrowed | 1. Land docs queue. 2. Collect aircraft.md §9 native observations before full owner/live-shadow credit. |

2026-10-26. Spec-only work; no engine, game or export files changed.
Specified non-finite x87 speed/time branches, runway sampling/weighted selection,
approach placement, fire-search geometry, rotor power/order, home completion,
startup phase gate, defaults/teardown, player refresh and goal/dispatch order.
Corrected “best clearance”, 500-unit maximum and nearest-fire summaries.
The three alleged ray probes are synthetic flight calibrations.
Added worked cases and conformance descriptions 28–35.
AF2, AF3, AF4, AF5, AF6, AF10, AF11 moved to questions/answered/aircraft.md;
question index regenerated. AF10 is static; the other six retain precise
runtime input/output observations in aircraft.md §9. Archival is not full
flight/AI/attachment/projectile/spotlight/message/enumeration verification.
AF11 distinguishes retained QI, borrowed rotor storage and creation ownership;
no complete enumeration API or deterministic scenario/group/exemplar claimed.
Sources: main-checkout re_query export, decomp ranges, instruction bytes and
file-backed numeric constants. No REA calls or runtime observation performed.
Gate: tools/doccheck.py check, tools/questions.py check, git diff --check passed.
No C ABI changes or proposed ABI rows. No cargo/clippy/ctest/game/shadow runs.
