| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gdriver-close12 (DX7 owner closure) | Codex / `core-gdriver-close12` (engine lane) | in progress | Implement settled RD6–RD11 contracts and gate |

## Handoff
- Goal: close nSGLDX7 owners in engine/core/src/gdriver, no ABI/plugin changes.
- Done: spec merge 356bc4ecf (generated total resolved preserving both branches' deltas).
- Implemented draft: lifecycle/defaults, context ranking/depth/rebuild, compact writes/quads, texture ranking/availability/copies, light/material setters, regions; DX7-19–49 recording tests added.
- Focused cargo session 53019 TERMINATED: 48/49 passed; only DX7-12 failed because the new depth CreateSurface was counted as a flip attempt. Filter corrected; subsequent helper tests/source changes NOT yet compiled.
- No running operations. Next: fix build-runs/format-owned.py stdout to UTF-8, run it, normalize CRLF in returned patch and apply via apply_patch (previous script patch was NOT applied because endsWith check expected LF).
- qbuild fmt (9 absolute paths) failed: 2918 diff lines. Script uses rustfmt on build-runs copies only, emits apply_patch; NEVER write rustfmt stdout to sources. Then rerun focused cargo tests and clippy diagnostics.
- Final required once stable: qbuild all; qbuild verify (coordinator native queue); update/commit report and merge queue only after successful gates.
- First focused compile failed (missing recorder variable), corrected; q.py unavailable (Git Bash CreateFileMapping error); direct cargo works (cargo test --manifest-path engine/Cargo.toml -p fc4-core gdriver:: -- --test-threads=1, ~3 min compile).
- README owner table updated with DX7-19–49 + lifecycle helper test names; audit it against actual tests. Additional helpers after 48/49 run are untested. No ABI/plugin changes.
- New RD12 appended only; QUESTIONS generated index intentionally left for coordinator regeneration (no tools/ scripts allowed).
- Open contracts: RD6 remaining converter arithmetic, RD7 CopyColorBuffer; new RD12 owner/context surface mapping; RD1/RD3/RD4 external meanings.
- Worktree intentionally detached; do not switch branches or update refs.
- No game requests; pure core only. Required final qbuild all and verify.
