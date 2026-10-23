| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-host5b: Lua host owner closure | Codex / core-lua-host5b | in progress | Implement settled LH4/LH5 contracts and verify full gate |

## Handoff
- Goal: close all settled cGZLua5, cGZLua5Thread and command-generator owner behavior; interpreter remains foreign.
- Done: implemented LH4 registry/thread lifecycle and LH5 setup, notifications, dispatch and cleanup; expanded owner table.
- Done: focused fixture suite reached 24/24; evidence build-runs/focused-20261023-211824.log. Earlier diagnostic failures preserved separately.
- Next: collect focused operation for latest registry-context/raw-borrowed-pointer correction, finish formatting, run stable full all gate.
- Current focused process: exec session 3641, build-runs/focused.py (collect the same operation).
- Thread views share synchronized state; explicit release/destroy methods handle foreign lifetime operations. No RefCell.
- Only pre-existing spec §15 Q1/Q2/Q5/Q8 remain unsettled; no new questions filed.
- No game requests; core-only task, no plugin/C ABI changes authorized.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-lua-host5b-r1-manifest.json.
