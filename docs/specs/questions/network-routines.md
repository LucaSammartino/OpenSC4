# network-routines: open questions
<!-- Questions about [network-routines.md](../network-routines.md). Append new entries at the END; when one is answered, move it to answered/network-routines.md. Rules: README.md. -->

## From the placement-demolition helper (re-network-routines4, 2026-10-23)

<!-- from: placement-demolition helper contract (2026-10-23) -->
- **NRT1 — invalid-domain and allocation guarantees:** [network-routines.md](../network-routines.md) §§2, 7
  specifies the ordinary in-city, initialized-service, valid-object, successful-allocation domain. Is there any
  caller-side invariant or allocator guarantee that extends it to signed-overflow coordinates, malformed ranges,
  null required outputs or failed region/bitmap/rect-vector allocation? The helper has no validation/overflow
  checks; region allocation null is published and can subsequently be dereferenced, whereas temporary-filter
  allocation null is explicitly forwarded to demolition. No total false-on-invalid rule is established. This
  does not block the ordinary-domain algorithm; do not invent memory-safety semantics for unsupported inputs.
  [unknown beyond the instruction paths documented in §7; no runtime probe requested]
