| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities-complete4: portable view family reconciliation | Codex / `core-view-utilities-complete4` (engine lane, detached HEAD) | in progress | 1. Reconcile 50-body native catalogue and independent vectors. 2. Implement and gate. |

## Handoff
Goal: reconcile portable SC4ViewUtilities with VU2–VU11; no ABI/plugin/runtime changes.
Done: da4fbb16 records independent native vectors before implementation. Reconciled signed rotation/edges, wrapping integers, constants/known numeric stores, plane/triangle line semantics and distance, cache sources, previous perspective corners, ordered clipping, checked resource iteration and generic index decoding.
Done: focused quick PASS (2730 core tests, dependent crates compile, CMake 4/4); five-file fmt PASS (canonicalization warning only). README maps all 50 distinct native bodies, independently checked against spec: 19748B, rotation78B, separate min/max174B. Added strict first-NaN AABB, aliased cross, row-matrix and checked model-abort vectors; raw negative strip index remainder corrected after review.
Done: f4967251 reconciles the 50-body checklist and focused edges. Full workspace tests 4718/0; clippy found const-from_bits MSRV and eight-argument frustum-helper errors. Fixed with exact 0.01f32 literal and bundled basis tuple (no arithmetic change).
Next: finish current all run (session78771), rerun final stable all after these fixes; record gate, replace this Handoff with report and queue wip because VU12/VU13/native numeric limits remain.
Facts: original core report is partial (38 described contracts), new inventory is 50 entries/19748 bytes including free rotation 78B and min/max 174B.
The missing bodies are helpers, not lifecycle/setters; no native utility Init/Shutdown/destructor exists in the catalogue. Native iterator destructor is a separate object.
No game requests; pure core scope. No external sources or subagents.
VU12/VU13 filed for missing numeric mode/store graph and extreme generic numeric index conversions. No full-owner/native-bit claim.
Baseline all: pre-change Rust tests and clippy passed; late ctest cargo test raced the independent test commit and failed on six then-unimplemented symbols. Evidence preserved in build-runs/baseline; this is not a stable full baseline PASS.
No build/game requests outstanding before final all launch.
q.py cannot start Bash in this environment (E_ACCESSDENIED); use bounded PowerShell reads and approved qbuild.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view-utilities-complete4-r1-manifest.json.
