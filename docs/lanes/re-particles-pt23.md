| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-particles-pt23 (particle owner contracts) | Codex / `re-particles-pt23` | ready; partial spec increment; doccheck passed | Implement PT13 boundary; continue PT1/PT2/PT8–PT11 |

## Report
RE output only: particles.md gains ordinary Init/Shutdown, transform and parameter contracts, zero defaults, and PO37–PO47 (11 proposed/unexecuted vectors).
PT3: all three seed scalars are positive-zero f32 in the PE zero-filled .data tail; the former pointer-like file reads were outside SizeOfRawData.
PT12: unsigned component-wise maximum at 005BA010; corrected rigid-body-2d.md §3 and RB-P1 to agree with particles.md.
PT13: one retained cISTETerrain, countX/width/countZ/width queries, f32 extents, repeated-Init fast path, release order, both transform records and both parameter channels.
PT3/PT12/PT13 moved to questions/answered/particles.md; regenerated QUESTIONS.md. No duplicate questions added.
Gate: direct `python tools/doccheck.py check` passed. The q.py attempt failed before doccheck (Git Bash CreateFileMapping error 5); it is not a failed source check.
No engine, plugin, capi, ABI, coverage-map or tool changes; no ABI proposals. Cargo/clippy/ctest and game/shadow runs are n/a for this RE-only delta.
Evidence: main-checkout Windows export, bounded decompilation reads and read-only PE section inspection; no REA call or game execution.
Limits: the requested complete owner specification is still unfinished. PT1, PT2 and PT8–PT11 remain open; mesh, decal, terrain/static-sector and full collision/joint numerics are not claimed settled.

## Handoff
Stop new RE work at the two-thirds budget cutoff; land this reviewable specification increment.
Next: prioritize PT11 AddObject/inertia/integration and PT10 terrain/static sectors, using rigid-body-2d.md as the existing solver contract; do not duplicate it.
PT12 invalidates the former minimum-sector implementation contract; PO47 additionally distinguishes unsigned half comparisons.
No game requests or external dependencies outstanding. Ignored evidence/helper: build-runs/inspect-particle-data.py; wrapper failure log under build-runs/q/.
