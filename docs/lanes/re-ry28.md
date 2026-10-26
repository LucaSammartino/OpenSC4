| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ry28 / RegionalCity RY9; kind docs | Codex; re-ry28 worktree; detached HEAD, reserved branch re-ry28 | ready; merged replace-region27 f52ce7a64; spec e568dea3c; ABI unchanged, no seam reservation; scoped docs PASS, cargo/clippy/ctest n/a; game n/a, shadow n/a; RY9 answered | 1. Wire replace-region27's native demolition host using regional-city.md §6.2.1. |

Resolved only RY9 in regional-city.md §6.2.1 and regional-city-records.md §4 introduction/§4.4/§4.6.
The border call uses actual demolition, privilege 0, flags 0x1, dezone false, null output/clicked pointers, and final coordinates 0, 0.
Its caller filter matches classified layer: surface 1, Subway 2, Pipe 3; type-only and property-holder queries accept all.
Documented system versus player locks, remaining view-dependent gates, separate PowerPole dezone, ignored result, and null-filter allocation fallback.
Added an admission table, a masked west-edge worked request, and a conformance case; preserved the existing placement contract.
Moved RY9 with provenance to questions/answered/regional-city.md, marked ANSWERED 2026-10-26 (re-ry28): MATCHES, and regenerated QUESTIONS.md.
Evidence: original call 0x004AAE73–0x004AAE96; construction 0x004AAE55–0x004AAE69; layer predicate 0x005E9E40; classifier 0x005E9CA0.
Used re_query.py and existing decomp ranges; instruction bytes verified numeric arguments and filter behavior. REA was not needed.
Scoped docs validation PASS: current index, RY9 archive placement, 29 existing local link targets, git diff --check.
Global questions.py check FAIL: sole diagnostic is pre-existing NT36 open/answered duplicate; checker and both affected files verified unchanged since f52ce7a64.
Validation helper: build-runs/check-ry9-docs.py. Cargo/clippy/ctest and in-game runs n/a for this RE documentation task.
No new engine/code edits or ABI proposals; no ABI-row file. The explicit prerequisite merge imported replace-region27.
Missing: downstream host implementation/registration remains the engine lane's work; no RY9 uncertainty remains.
