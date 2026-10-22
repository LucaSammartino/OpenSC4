| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities-complete4: portable view family reconciliation | Codex / `core-view-utilities-complete4` (engine lane, detached HEAD) | in progress | 1. Reconcile 50-body native catalogue and independent vectors. 2. Implement and gate. |

## Handoff
Goal: reconcile portable SC4ViewUtilities with VU2–VU11; no ABI/plugin/runtime changes.
Done: reserved worktree reused; baseline qbuild all reached CMake build after Rust/clippy. Independent §11.10–15/18 expectations committed before implementation (signed rotation, partial writes, tolerance, negative-t metric, NaNs, independent cache counts).
Next: implement reconciled numeric/geometry/cache/resource behavior and complete the per-body checklist.
Facts: original core report is partial (38 described contracts), new inventory is 50 entries/19748 bytes including free rotation 78B and min/max 174B.
The missing bodies are helpers, not lifecycle/setters; no native utility Init/Shutdown/destructor exists in the catalogue. Native iterator destructor is a separate object.
No game requests; pure core scope. No external sources or subagents.
q.py cannot start Bash in this environment (E_ACCESSDENIED); use bounded PowerShell reads and approved qbuild.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view-utilities-complete4-r1-manifest.json.
