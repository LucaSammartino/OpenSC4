| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-inputcontrols4 (pure core view input tools) | `Codex` / `core-view-inputcontrols4` (engine lane) | in progress | Implement published supported owners, fixtures and inventory; full gate. |

## Handoff
- Goal: new production `engine/core/src/view_input_controls/`, published sections 1-8/10 only.
- Done: independent fixtures.md written first; initial production scroll, level, label, sign, demolition and filter algorithms implemented.
- Done: reused current ViewUtilities/Pixel/Vec3 and Terrain APIs; no native owner pass-through.
- Next: expand executable fixtures, inspect baseline result, finish method/byte inventory, questions and focused verification.
- Baseline `qbuild all` session 31106: cargo log complete, clippy error seen; preserve and classify first diagnostic before final gate.
- No ABI changes, game requests, native-exact claims, or full base-control ownership.
- q.py cannot launch Bash here (E_ACCESSDENIED); use native PowerShell with capped output.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-view-inputcontrols4-r1-manifest.json`.
