| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-scriptserver4: native ScriptServer boundary | Codex / `glue-lua-scriptserver4` (engine, detached) | in progress | 1. Establish published factory/service and native lifetime route. |

## Handoff
- Goal: wire only the completed cGZLuaScriptServer owner into native execution if published contracts support insertion.
- Done: initial handoff committed 6fec4c86; public acquisition/SDK audit found two concrete binding blockers, filed LH1/LH2 in docs/specs/questions/lua-host.md.
- Next: finish baseline qbuild all (session 86646), reconcile measured results, publish blocked report and wip docs queue.
- Facts: main C:/Users/tlori/FreeCity4; this lane must not switch branches or update shared refs.
- q.py failed because Bash service access is denied; use capped PowerShell reads and commands.
- No native glue, ABI edits, game requests or replacement claims yet.
- Spec 1.2 lists VM/thread/generator registrations, no ScriptServer registration; 1.4 names app AddSystemServices without insertion call sequence.
- Spec 6.3.2 leaves constructor argument roles ambiguous; SDK constructor is (serviceID, signed priority), core constants label them CLASS_ID/SERVICE_ID. Do not bind either guess.
- Required command-system accessor and generator/target native ABI are not published in SDK; LH2 requests exact foreign endpoints.
- The questions index is generated; prohibited tools/ scripts are not run and index is left for coordinator regeneration.
