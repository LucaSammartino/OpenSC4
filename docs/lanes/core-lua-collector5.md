| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lua-collector5: Lua extension collectors | Codex / core-lua-collector5 | in progress | Map collector requirements and implement owner closure |

## Handoff
- Goal: close all cLUAExtensionsCollector owners in the existing core Lua host; no plugin, C ABI or VM implementation.
- Done: plan committed as 7d8a3c01; implemented both collectors, ordered 47-entry process registry and foreign registration consumer; README owner-closure table added.
- Tests: five conformance integration tests for section 8 and section 16 T9 passed (5/5). Explicit new-file formatting check passes. q.py wrapper failed before launch due to Windows Bash CreateFileMapping access failure; direct focused Cargo invocation passed.
- Next: run qbuild all once on stable final source; implementation commit contains this handoff.
- Constraints: clean room; game services and VM remain foreign traits; final qbuild all required.
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-lua-collector5-r1-manifest.json.
