| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzservices4: framework GZ services spec (framework-gz-services.md) | `re-gzservices4` / (RE lane) | in progress | 1. see Handoff |

## Handoff
- goal: spec the largest concrete GZ framework-services owner family (exact owners by bytes) in docs/specs/framework-gz-services.md; questions for gaps.
- done: spec docs/specs/framework-gz-services.md (cGZMessageServer2 2780 B, cGZMessageQueue2 1941 B, cGZMessageQueueManager2 951 B) + questions GZ1-GZ4.
- next: cGZMessageServer (v1, 5796 B, iface rows in docs/abi/interfaces.tsv cIGZMessageServer) as spec §7; then cGZMessage2CommandDispatcher (666+203 B).
- facts: REA down (no app open) -> export asm only. GZ owner ranking (bytes): MiscCommandSystemSupport 15356, Framework 6714, nGZLog::cService 6041, MessageServer 5796, ToolCommandSystemSupport 4450, FileSystem 4215, COM 3523, CommandServer 3320.
- facts: proposed abi rows build-runs/abi-rows-re-gzservices4.tsv (Queue2 slot 5-10 renames, QueueManager2 vtable 00ad1b80). Helpers build-runs/asmc.py (compact asm), vt.py (read vtable dwords).
