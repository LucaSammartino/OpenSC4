| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ss29b sound-system; kind docs | Codex; re-ss29b worktree; reserved re-ss29b branch, detached HEAD | ready: SS5–SS8 answered; commits a5c861853, 15c0b5d2e, 34fd2e004, 5963950e8; doccheck passed; ABI false; game n/a; shadow n/a | 1. Coordinator reviews/lands docs. 2. Engine owners consume worked cases. |

Answered SS5–SS8 in sound-system.md §§5.2, 6.1, 6.2 and 7.1.
Moved entries to questions/answered/sound-system.md and regenerated QUESTIONS.md.
Confirmed formats/ownership, selection/fades, cell state/controller behavior,
period-20 classification, >250-ms timing, extensions and shutdown order.
Preserved the consuming event chain. No engine or ABI files changed.
Gate: tools/doccheck.py check passed directly in PowerShell; git diff --check passed.
The q.py validation attempt failed to launch Bash (Win32 error 5);
the subsequent direct doccheck completed successfully, exit 0.
No game runs, cargo, clippy or ctest: documentation-only RE lane.
Evidence: local export/decomp and instruction bytes; no REA calls/errors.
Runtime limits: malformed legacy-record failure/crash and backend-dependent
factory/timer outcomes have explicit observation instructions in the spec.
ABI proposals: none. Unrelated open sound-system questions remain untouched.
Resume corrected two stale overview sentences that still called SS7/SS8 open.
Queue: build-runs/merge-queue/re-ss29b.json, kind docs, actual committed HEAD.
Next: coordinator reviews/lands these docs; implementers use worked cases.
