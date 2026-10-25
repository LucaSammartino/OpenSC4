# Archived HANDOFF §0 (replaced at the run-5 end, 2026-10-25)

## 0. Current state — USER STOP, 2026-10-23 11:10 BST

**STOP is set** (`python tools/lanes.py stop`). Nothing runs: the nine stopped workers are landed and
the refiller has exited. Clear STOP only when the user explicitly resumes.

**Main:** the nine stopped workers were finished and landed as train124 (8 lanes, [run 301](runs/train124-301.md))
and train125 (`core-my-street-sim4`, [run 302](runs/train125-302.md)); both gates and regresses were clean (the
train124 netmgr 72/73 -> 71/73 dip was run noise, back to 72/73 in train125). Every lane tip is an ancestor of
main. The new core modules are unwired, so coverage did not move. No batch is in flight.

**Coverage** (`tools/coverage.py`): specified 57.8%, implemented 36.1%, verified 19.7%, replaced 18.1%
(3,226,906 / 2,018,463 / 1,099,943 / 1,012,661 of 5,584,842 bytes). Over the 12 hours ending at this stop
(trains 103-123, ~590 commits, +103k lines) this moved by +8.4 KB implemented, +64 KB verified and 0 replaced.
The exact-function credit path (76 B, self-reviewed) was removed in the 2026-10-23 trim. Bounded/partial deliveries earn no owner credit; on resume, prefer finishing owners over
parallel partial slices.

**Unlanded work** (worktrees removed 2026-10-23; every unlanded branch is pushed to origin, and each removed
worktree's build-runs text is in `build-runs/worktree-archive-2026-10-23.tar.gz`, local only):
- `re-curve-editor-views4`: spec transliterates x87 instruction sequences (CONTRIBUTING.md "Specs"); rewrite
  as behaviour before landing.
- Held Codex lanes (`git branch --no-merged main`, about 35): mostly blocked on unpublished contracts; review
  one by one before reviving. `tools-build-singleflight4` and `tools-refill-rea-admission4` extend the removed
  swarm tooling.
- Pre-run parked branches (`glue25`, `glue26`, `glue-terrain3`, `verify-netmgrfix4`, `integration/*`,
  `coordinator/*`) are unchanged.

**Known issues:** train121 really worsened LandValue (72/72 to 71/72) and the coordinator landed it anyway
([incident](ops/acceptance-incident-train121.md)); every landing must read the full comparison delta, not just
the verdict. Native per-step LandValue capture is still 0. C: is 68% full after the worktree cleanup.

The stop note this replaces is archived verbatim in
[handoff-user-stop-2026-10-23-0916.md](history/handoff-user-stop-2026-10-23-0916.md).

