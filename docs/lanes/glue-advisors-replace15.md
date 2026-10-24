| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-replace15 (advisors replacement registration) | `Codex` / `glue-advisors-replace15` (engine lane) | in progress | (1) Register factory and test; (2) gate; (3) run replacement games to 2032. |

## Handoff
- Goal: default-off advisors replacement factory, registration ctest, clean replacement games through 2032.
- Done: b592c83e4 adds default-off advisors registration and plugin tests for disabled enumeration and enabled factory/QI/release.
- Next: run required stable-source all gate (including queued native verify), then advisors-only and combined replacement games.
- Gate: qbuild all is still running (session 92127); Rust compiling core, conformance, formats, capi. No failure or terminal result yet. Collect this same operation.
- Full profile source: docs/runs/s8n4-c40-45d931a9-20261022b.md lists current40 plus span8; docs/runs/train210-643.md adds dispatch, exemplars, span1555, span4444, span565, span8, span8888, tutorial to current40 (48 keys). Simulator/budget default-on.
- No game requests submitted. Wait helper: build-runs/wait-verdict.py (one blocking operation; do not use qwait).
- Constraints: clean-room; do not run/read tools; use lane build queue and game queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-advisors-replace15-r1-manifest.json.
