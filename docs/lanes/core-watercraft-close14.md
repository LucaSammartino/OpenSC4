| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close14 (complete cSC4Watercraft owner closure) | `Codex` / `core-watercraft-close14` (engine lane) | in progress | (1) Close all settled owner rows; (2) run full and native gates. |

## Handoff
- Goal: close every settled cSC4Watercraft owner row in engine/core/src/watercraft, with conformance tests; no plugin/C ABI changes.
- Worktree reused at detached HEAD; clean at restart. Prerequisites already merged: 4ee9852ce (core-watercraft-close12), f075b206b (re-watercraft-wc13).
- Next: audit module README and owner/runtime code against settled spec; implement remaining lifecycle/models/publication and conformance vectors.
- No gate or game request started. No plugin/C ABI work; game shadow requires an adapter outside scope.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-watercraft-close14-r2-manifest.json.
