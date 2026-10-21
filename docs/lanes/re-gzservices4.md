| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzservices4: framework GZ services spec, messaging family (framework-gz-services.md) | `re-gzservices4` / (RE lane) | ready, no ABI change; spec commits `156e0877` + final; owners cGZMessageServer2 2780 B, cGZMessageQueue2 1941 B, cGZMessageQueueManager2 951 B, cGZMessageServer 5796 B (11468 B total); GZ1-GZ4 filed; no game runs | 1. coordinator: map owners cGZMessageServer2, cGZMessageQueue2, cGZMessageQueueManager2, cGZMessageServer to spec docs/specs/framework-gz-services.md (spec path only, no core). 2. abi lane: apply build-runs/abi-rows-re-gzservices4.tsv (Queue2 slot 5-10 renames, QueueManager2 vtable 00ad1b80). 3. next RE round: cGZMessage2CommandDispatcher (666+203 B), cGZMessageQueue v1 (77 B), then cGZCommandServer (3320 B). |

## Report

**What changed.** New spec docs/specs/framework-gz-services.md and questions file
docs/specs/questions/framework-gz-services.md (GZ1-GZ4); QUESTIONS.md regenerated.

**Owners specified (exact, from tools/coverage.py --owners at main ab23a7fa, subsystem "framework services (GZ)",
75,592 B total):**

| Owner | Bytes | Spec section |
|---|---|---|
| cGZMessageServer | 5,796 | §4A |
| cGZMessageServer2 | 2,780 | §4 |
| cGZMessageQueue2 | 1,941 | §2 |
| cGZMessageQueueManager2 | 951 | §3 |

Total 11,468 B (15.2% of the GZ services subsystem). Not claimed: the rest of the GZ family (command system,
framework, log, COM, file system, date, script interpreter, gimex, etc.).

**Contracts covered.** Class ids / service ids / iids / tick priority; layouts and constructor defaults; every
vtable slot of Queue2 (25), QueueManager2 (9) and the cIGZMessageServer2 (14) and cIGZMessageServer (12)
interfaces; notification registry with deferred (pending) add/remove and flush guard; synchronous send
(snapshot then deliver outside the lock; v2 wildcard type 0 second pass; v1 no wildcard, type 0 ignored, 385
target snapshot cap); post queue and tick (64-bit taken counter, drain-all flag default true); directed messages
(deque, countdown delivery); CreateMessage (clsid 0x46DF45E7 -> cRZMessage2Standard); worked example.

**Evidence.** REA was unavailable (no app open in the shared session): all from the export's instructions
(re_query.py asm) and vtable dwords read from bin/SimCity4.exe. docs/abi Queue2 names for slots 5-10 do not match
behaviour; proposed renames and new QueueManager2 rows in build-runs/abi-rows-re-gzservices4.tsv.

**Questions.** GZ1 (server2 slot 12 counter identity), GZ2 (thread-signal wait unit), GZ3 (is the pending list
reachable), GZ4 (callers of directed-message slots).

**Next.** cGZMessage2CommandDispatcher (+Message) and cGZMessageQueue v1; then cGZCommandServer /
cGZCommandParameterSet; largest remaining GZ owner is cGZMiscCommandSystemSupport (15,356 B, one 7,226 B function).
Helpers kept in build-runs/: asmc.py (compact multi-function asm), vt.py (dump vtable dwords).
