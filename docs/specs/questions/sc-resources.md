# sc-resources: open questions

<!-- Questions about [sc-resources.md](../sc-resources.md). Answered entries move to answered/sc-resources.md. -->

No open questions. SCR1–SCR5 and SCR7 were answered by re-scres-scr5;
SCR8/SCR9 were answered on 2026-10-24 by re-scres-scr8 in spec section 10.
SCR10 was answered on 2026-10-24 by re-scres-scr10 in spec section 11.
See [answered entries](answered/sc-resources.md). Runtime coverage remains
separate from these answered native contracts.

<!-- from: glue-persist-shadow8 (2026-10-24) -->
- **SCR11 - live buffer resource independent replay inputs:** what public read-only observation contract associates a currently cached buffer resource with its original encoded bytes, target color type/create flags, load cursor, and subsequent drawing/mutation inputs? Section 7 specifies loading and buffer delegation, but enumerating a cached key does not establish that its present pixels equal a fresh decode of the segment's bytes. A cache lookup can also load on a miss; buffer locking can alter state. Please define safe acquisition and pixel-read/concurrency preconditions and a replayable live domain. Do not seed core with native pixel answers and call it a content comparison. Buffer content shadow remains disabled pending this contract; manager cache type projection can proceed independently.
