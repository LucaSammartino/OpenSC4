| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-paths-contractfix4: PA4-PA8 portable path corrections | `Codex` / `core-paths-contractfix4` (engine lane) | in progress | Audit published contracts, correct portable paths, run full gate |

## Handoff
- Goal: apply PA4-PA8 to production paths and directory-entry lifecycle; reuse cRZDirectory.
- Scope: portable Rust only; no ABI/shared files, native glue, game runs, or whole-owner completion claim.
- Done: first substantive PA4 correction preserves the full counted ResolvePath input on the full-prefix branch; independent NUL/copy vector added.
- Baseline qbuild all launched; logs in build-runs/qbuild-*.log (result pending).
- Next: add explicit CRT classifier boundary, correct SplitDirectory, consolidate directory owner and entry lifecycle; run quick gate.
- PA1/PA2/PA3 remain open; preserve selected normalization rule.
- Correction reservation logical round 2/3; this task permits one round only.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell output and required qbuild wrappers.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-paths-contractfix4-r1-manifest.json.
