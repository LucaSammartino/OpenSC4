| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrecords-mutations4: LR10/LR11 native mutation contracts | Codex / `re-lotrecords-mutations4` (RE docs, detached worktree) | ready; LR10/LR11 answered; 27 native vectors; documentation diff check passed; cargo/clippy/ctest and game n/a; ABI unchanged | 1. Coordinator consumes contracts for concrete lot binding. |

## Report

Answered only LR10/LR11 in lot-records.md §3.2.1–3.2.2/13.2, with independent native-derived
vectors T31–T57 in §17.5. Reconciled lot-developer.md §3.2/6.4, moved both question entries to
questions/answered/lot-records.md with dated DIFFERS rules, and regenerated QUESTIONS.md.
LR6/LR7 remain open; LR8/LR9 were not re-investigated.

SetPopulation clamps an unsigned word to available capacity, selects the first matching profile
capacity-map index, and assigns/inserts in that existing population map. It never creates a missing map.
AL is explicitly false/true; full EAX is not defined. Zero/unchanged assignments still recompute wealth.
That helper inserts absent capacity keys with zero counts, potentially across several population maps.

Published exact capacity fraction/rounding, occupancy capacity means and x87 store points, wealth
fallbacks, zero-score behavior and descending positive ties. Unknown native types create a purpose-zero
profile with empty capacities. An empty vector is distinct from a vector containing an empty map:
the latter faults at integer division 006C1258. Missing corresponding population-map indices are
unchecked native invariants; there is no invented successful result for them.

SetState now has full day/resource/population/wealth/power/copy/message order. State comparison uses
the whole argument before storing its byte. No success boolean is defined; nominal changed-path EAX=1
comes from final copy Release. Snapshot follows all synchronous updates, including power. Population
and travel-map trees copy independently, properties/configuration retain references, and unit/travel
references and trips start empty. Queue → message → copy ownership and final reference balance are
specified. No valid-container LR10/LR11 behavior gap remains; this does not authorize native-layout forwarding.

Evidence: export queried first with re_query.py; bounded decomp reads were cross-checked against
instruction bytes for 006C1940, 006C3100, 006C1430, 006C2450 and dependencies. Queue retention is
0092D0BB → 0092DDD2 → 007E98E5; copy retention/release is 005447C7 / 009133C9. PE data reads
confirmed 00B49114=0.0f, 00A81228=1.0f and 00AB16B4=12582912.0f. No REA request was needed.
q.py could not launch Bash (E_ACCESSDENIED); bounded local PowerShell queries completed the work.

Milestones: faa5a656 established rules/vectors; 0caae53a completed arithmetic/lifetime and answered
the questions; this final report commit adds the explicit nominal SetState return-register observation.
The worktree is intentionally detached; the supervisor publishes commits to the reserved branch.
Git emitted a packed-refs.lock permission warning but both milestone commits completed successfully.

Validation: questions index regeneration succeeded; git diff against the starting commit passed
--check; only the six intended documentation files changed. Index delta is exactly LR10/LR11
open→answered and the corresponding totals. The 27 vectors are specifications, not executions.
Cargo/clippy/ctest and game runs are n/a. No engine reads/edits, ABI-table edits, ABI proposals,
exports, runtime activation or coverage claims. No game requests outstanding.

Next: coordinator can use these contracts for the withheld concrete slot-34/55 owner binding.
