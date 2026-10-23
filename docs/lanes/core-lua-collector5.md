| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-collector5: Lua extension collectors | Codex / core-lua-collector5 | in progress | Map collector requirements and implement owner closure |

## Handoff
- Goal: close all cLUAExtensionsCollector owners in the existing core Lua host; no plugin, C ABI or VM implementation.
- Done: reserved detached worktree reused; working tree initially clean.
- Next: read lua-host sections 8 and 16 and existing Lua host code, then implement collector behaviours and conformance coverage.
- Constraints: clean room; game services and VM remain foreign traits; final qbuild all required.
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-lua-collector5-r1-manifest.json.
