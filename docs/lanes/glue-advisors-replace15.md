| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-replace15 (advisors replacement registration) | `Codex` / `glue-advisors-replace15` (engine lane) | in progress | (1) Register factory and test; (2) gate; (3) run replacement games to 2032. |

## Handoff
- Goal: default-off advisors replacement factory, registration ctest, clean replacement games through 2032.
- Done: default-off advisors registration added; plugin tests cover disabled enumeration and enabled factory/QI/release.
- Next: run required stable-source all gate (including queued native verify), then advisors-only and combined replacement games.
- Constraints: clean-room; do not run/read tools; use lane build queue and game queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-advisors-replace15-r1-manifest.json.
