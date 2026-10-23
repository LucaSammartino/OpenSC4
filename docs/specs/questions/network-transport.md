# Open questions: network-transport.md

**Q11 (re-network-transport4, 2026-10-23) — lookup initialization and owner identity**
network-transport.md §7 contracts normal initialized globals only. Which primary construction/
startup bodies initialize the lookup object's interface/refcount and the global critical section,
queue, cache, counter and worker flags, and which are uniquely attributable to this owner?
Adjacent unnamed constructors are not sufficient evidence. No reopening of capped web owners.

**Q12 (re-network-transport4, 2026-10-23) — notification and timer boundary**
network-transport.md §§2/6: qualify socket SetupWindow/ShutdownWindow, Notification,
StartAsyncCustomNotification and stream DoMessage state/callback order, including actual
message ABI and target replacement ownership. Timer setup and raw callback are specified;
timer scheduling/cancellation and racing close semantics remain foreign, not assumed rollback.

**Q13 (re-network-transport4, 2026-10-23) — residual adapters and public address construction**
network-transport.md §5 specifies transparent raw 16-byte records and adjacent raw address/port
adapters. Which public host-order address/port constructors and getters, folded handle/error
getters, and option/time-limit wrappers belong uniquely to these owners? Supply primary
width/association evidence before extending support. Generic clock epoch/units and string
normalization outside the bounded ASCII domain remain foreign dependencies; do not analyze
excluded NetworkFileCache or HTTP/FTP algorithms to fill those dependencies.

**Q14 (re-network-transport4, 2026-10-23) — worker completion scalar source**
network-transport.md §7.3: instruction 0081e25e reads the original callback-argument stack
slot for message virtual +30, not the saved queued ID. That slot's address was passed to
foreign string-copy construction at 0081e1aa–0081e1b7, after its original index allocation
was freed. What is the exact normal value/ownership contract of that scalar? Keep full
message payload equivalence unsupported until the calling convention and foreign scratch
write contract are proved. Do not infer ID semantics from a setter name or pointer goldens.
