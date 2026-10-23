# animation-tick-manager: open questions
<!-- Questions about [animation-tick-manager.md](../animation-tick-manager.md). Append new entries at the END. -->

<!-- from: re-other-simulation4 (2026-10-23), original round 2/3 -->
- **TK1 — sections 3.2–3.3 and 7: framework retention and external tick caller.** What concrete framework implementations receive Init slots 3/9 and Shutdown slots 10/4, how many manager references do they retain/release, and which indirect caller invokes primary slot 9? Init `00448044–00448054` and Shutdown `0044850C–0044851C` establish the sequence only. The +4 receiver `006B8AA0` reads a message type and returns true without ticking. Effects `00594EE1–00594EF3` calls slot +0x24 on an embedded object, not EnableAutoStep +0x38 on this manager. [Unknown external ownership/cadence; do not infer scheduler or callback equivalence.]

<!-- from: re-other-simulation4 (2026-10-23), original round 2/3 -->
- **TK2 — section 2: persistence dependency.** Establish the complete stream schema, record-to-bucket placement, duplicate loading and reference transfer for write `00448650` and read `00448AB0`, including SC4ReadGZSerializable `005D9730` / SC4WriteGZSerializable `005D99A0`. Version 1 and final stream-error negation are established; the rest is partial. [Do not implement a guessed format or treat loaded entries as ordinary Add calls.]

<!-- from: re-other-simulation4 (2026-10-23), original round 2/3 -->
- **TK3 — sections 3.3, 4.2 and 9: destructive/exceptional reentrancy.** Determine supported behavior, if any, for Shutdown/final Release inside DoMessage, target AddRef/Release reentry during FIFO drain, and exceptions from delivery or draining. The ordinary in-tick guard and deferred DoMessage mutation are specified; drain `00448730` runs after clearing that guard, Shutdown breaks the message cycle before traversal, and catch `00448A7E` drains then calls runtime `009EF84F`. [No safe teardown, rollback or exception swallowing guarantee; retain caller lifetime precondition.]
