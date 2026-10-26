| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gc30 — growth capture questions; kind docs | Codex; .claude/worktrees/re-gc30; reserved branch re-gc30 (detached HEAD) | ready; spec commits 6da45ab59, 8907e7f51; doccheck passed, diff whitespace passed; cargo/clippy/ctest n/a; game n/a; no ABI exports or ABI rows proposed; MS-GC29-1, DM-GC29-1, AU-GC29-1, GD-GC29-1 answered/narrowed | 1. Apply capture contracts in growth-cities29. 2. Observe startup/history/aura boundaries. 3. Obtain a clean 2027-12-03 state-18 trace. |

- The requested docs-only checkout from growth-cities29 already matched this base; no engine files were read or changed.
- mysims.md §3.1.1 / advisors.md §3.1: system lastDay offsets and recipient entry/exit boundaries; startup monthly calls and late advisor registration worked cases.
- demand.md §8.1.1: uninitialized shared maximum-output slot; false history queries preserve it; nonempty series with empty windows evaluate endpoints. Controlled 0/512 cases yield 0.300000012/−3.5.
- aura.md §14.3.1: live occupant input capture, direct PostCityInit rebuild paths, permanent-grid checkpoints and three worked cases.
- growth-developer.md §5.2.1: state/iterator/grid layouts, per-tract reads, native callback effects and a controlled 85/226 unequal-slice case.
- All four entries moved to their named questions/answered archives with dated answers; questions.py index ran after each pair.
- Evidence: local export queries and main-checkout decompilation; numeric/offset/order claims checked against instruction bytes. REA was not needed.
- Remaining observations: original startup recipient trace; history maximum bits including false returns; grown-Lua recipient inputs and source grids; clean late-date state-18 per-tract trace.
- No new game runs. Reported exact prefixes and later corrupt/crash outcomes remain distinct; no historical mismatch is claimed fixed.
- Validation: doccheck check passed after both spec milestones; git diff --check passed after removing a trailing blank line.
- Git emitted a shared packed-refs lock permission error during commits, but each detached commit succeeded and HEAD was verified; shared refs were not modified intentionally.
