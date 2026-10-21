# network-manager: open questions
<!-- Questions about [network-manager.md](../network-manager.md). Append new entries at the END; when one is answered, move it to answered/network-manager.md. Rules: README.md. -->


<!-- from: re-netmgr-rng4 (2026-10-21) -->
- **NM13 - section 8.5 *[compat]* note and section 17.3 (draw order within one `ProcessUpdateList` pass):** exact
  replay of the §17 draws needs the original's key order when two or more cells of one pass draw. `pending` is a hash
  set at manager `+0x74` (bucket vector begin `+0x78`, end `+0x7C`, element count `+0x84`, read at
  `0x0060FA14`–`0x0060FA3E`). Needed: its initial bucket count, the growth rule (prime list, when it grows), the
  order of nodes within a bucket after insert and after a rehash, and whether the bucket count (which persists after
  the set empties) must itself be part of the §17.3 capture. [current: ascending key order *[choice]*]
