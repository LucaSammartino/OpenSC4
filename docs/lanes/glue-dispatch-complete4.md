| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dispatch-complete4 (dispatch replacement completion) | `Codex` / `glue-dispatch-complete4` (engine lane) | in progress | Complete published dispatch host contracts and measure native/replacement exercise |

## Handoff
- Goal: finish supported dispatch replacement, including vehicle views and independent source/pylon comparisons; no completion credit with missing contracts.
- Reused reserved detached worktree; initial HEAD equals main, clean status.
- Baseline qbuild all: cargo workspace and clippy logs clean; full ctest 67/67.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell commands.
- Implemented independent ground/air source registration and slot 15/21 comparisons; real station callback releases; automaton despawn/go-home/group callbacks; active-record shadow identity/ref fixes.
- Added script dispatch type: real station, BeginTool/Dispatch/live record/shadow/Revoke/EndTool; not success unless a vehicle actually exists.
- qbuild quick PASS (19/19 build edges); full final gate and original/replacement game exercises next.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dispatch-complete4-r1-manifest.json.
- Original game label gdc4-original-20261022-a1 to 2032 queued next; replacement gdc4-replace-20261022-a1 afterward, never concurrently.
- DP10-DP12 in docs/specs/questions/dispatch.md: missing native vehicle-view, pylon construction/placement, BeginDispatch ABI. Completion blocked on contracts; no ABI export changes.
