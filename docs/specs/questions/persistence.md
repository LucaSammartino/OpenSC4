# persistence: open questions
<!-- Questions about [persistence.md](../persistence.md). Append new entries at the END; when one is answered, move it to answered/persistence.md. Rules: README.md. -->

<!-- from: re-persistence4 (2026-10-21) -->
- **PR1:** (§3.1) which native-order flag do the DBPF packed-file records (`cGZDBRecordPackedFile` constructor
  `0x0096D859`, called from `0x0096D8D0`, `0x0096D9D1`, `0x0096DA30` and the two write-record constructors) receive
  from their segment, and can any segment in the shipped game be configured big-endian? [expected: always 1 for
  the shipped packages and saves, which are little-endian]

<!-- from: re-persistence4 (2026-10-21) -->
- **PR2:** (§6) does any shipped package or save contain entries of type `0x62B9DA24` (buffer resources, forms
  v0/v1/v2), and which forms occur? A DBPF index scan of the install answers it. [expected: few or none; UI images
  are PNG (`0x856DDBAC`) and load through the persist-buffer / gimex factories, not here]

<!-- from: re-persistence4 (2026-10-21) -->
- **PR3:** (§6.2 v1) the original's padded-stride copy swaps the source and destination steps. FreeCity4 chose
  the correct copy (identical whenever stride = packed row size). Confirm no shipped data or caller depends on a
  padded v1 load. [the choice stands]

<!-- from: re-persist-manager4 (2026-10-22) -->
- **PR4:** ([persistence-manager.md](../persistence-manager.md) §9, service update return) Which shipped
  callers observe the return byte of the manager service's slot 11 (`0x0097356D`), and what deterministic
  return should the clean-room implementation expose? Native instructions OR every cache update result
  into an uninitialized stack byte `[EBP-1]` (`0x0097359C`) and return it (`0x009735AF`); the empty-cache
  path is uninitialized too. Callback order/arguments are specified, but an exact boolean baseline cannot
  be inferred from this body. [expected: callers ignore it; verify call sites before choosing an initialized
  OR reduction or constant true; neither choice is established by this audit]
