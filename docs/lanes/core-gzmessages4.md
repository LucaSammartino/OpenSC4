| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzmessages4: pure-Rust core for GZ messaging owners (cGZMessageQueue2, cGZMessageQueueManager2, cGZMessageServer2, cGZMessageServer) | `core-gzmessages4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: engine/core/src/gz_messages/ from docs/specs/framework-gz-services.md §2-§4A; crate tests, clippy, full gate; no capi/plugin, no game run.
- Done: worktree created from main 5f835ead.
- Next: write queue/manager/server2/server1 modules + tests, qbuild quick, commit.
- Facts: host-token design: messages/targets/queues are opaque handles; refcounts via host callbacks.
