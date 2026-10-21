| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzmessages4: pure-Rust core for GZ messaging owners cGZMessageQueue2, cGZMessageQueueManager2, cGZMessageServer2, cGZMessageServer (framework-gz-services §2-§4A) | `core-gzmessages4` / (engine lane) | ready: core 8583ab66 + 3fbb7e94, questions 4e688f6d; no ABI, no capi/plugin/CMake; gate cargo 3628 passed / 0 failed (baseline 3614), clippy 0, ctest 62/62; no game run (core only, game false); questions GZ5-GZ7 filed. Partial: 4 of the 122 GZ owners; GZ1 slot 12 counter not implemented | 1. glue: map Token to COM pointers and wire a shadow of cGZMessageServer2 send/post/tick order. 2. answer GZ1-GZ7, then revisit [choice] sites. |

## Report

What changed (engine/core/src/gz_messages/, `pub mod gz_messages` in engine/core/src/lib.rs):
- `queue.rs` `MessageQueue2` (§2): all 25 slots by behaviour (push front/back with the empty-to-non-empty signal,
  remove first/last/all by type or equality, peek/get front/back/anywhere, blocking gets through a host
  `wait_signal(queue, 1_000_000)` callback, count, own refcount with Release destroying at 0 or 1).
- `manager.rs` `QueueManager2` (§3): sorted id -> queue map, Get/Add/Create/Remove(queue)/Remove(id)/RemoveAll,
  queue references via `QueueHost`.
- `registry.rs`: notification registry shared by both servers (§4.2): depth, pending list, flush with re-entry
  flag, apply-pending, destruction. Target releases receive the server, so a target destructor can re-enter
  (the only reachable pending path, cf. GZ3).
- `server2.rs` `MessageServer2` (§4.1-§4.7): send (snapshot type list + wildcard), post/tick with the 64-bit
  taken/limit and drain-all, directed deque with the shared countdown, CreateMessage (clsid 0x46DF45E7 via host),
  destroy.
- `server1.rs` `MessageServer1` (§4A): value records, no wildcard, type 0 ignored, snapshot AddRef/Release per
  delivery, 385-target limit.
- 14 tests: §5 worked example in both drain modes, ordering/duplicates/wildcard-twice, registration during
  delivery, re-entrant release through pending+flush, directed cancel/next-tick/countdown -1, ref balances to 0.

Measured: qbuild all PASS: cargo test 3628 passed / 0 failed (baseline 3614), clippy 0 diagnostics, CMake build
PASS, ctest 62/62. fmt clean on the 7 files.

Not done / honest scope: no glue, no shadow, no game run. GZ1 (slot 12 counter) not implemented; GZ2 wait unit
left to the host; GZ4 unknown. Locks are not modelled (single-threaded `&mut self`).

Questions filed (docs/specs/questions/framework-gz-services.md): GZ5 (server 1 directed storage/refs), GZ6
(reference timing: server 2 snapshot, removal at depth 1, directed release order), GZ7 (equality call direction).
