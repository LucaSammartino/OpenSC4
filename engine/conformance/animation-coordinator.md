# Animation coordinator vectors

These use format 1 with `"domain": "animation-coordinator"`. The top-level
`city_cells_x`, `zoom`, `visible` (world metres, min x/z then max x/z) and
`occupants` describe foreign state. Each occupant has `id`, `position` in metres,
`flags`, model class IDs, exemplar property IDs and optional `powered`.
Integer IDs can be hexadecimal strings. All eligibility is computed by the core;
the fixture supplies no eligibility callback and no unknown budget predicate.

Steps: `init`, `shutdown`, `set` (`id`, `on`), `message` (raw `id` plus typed
payload), `refresh_enabled`, `view` (`rect`), `power` (`id`, `on`), `detail`
(`byte`), `fail` (`target`, `add`, `remove`, `subscription`), `clear`, and `expect`
(`state`). Unknown operations and state fields fail. `error` on an operation must
equal the explicit core error's debug representation; unexpected errors fail.

State fields: `initialized`, `enabled`, `zoom`, `side`, `registered`, `ticks`
(sorted IDs), `flags` (sorted `[id, flags]` pairs), `blocks` (nonempty `[x,z,count]`
triples in row-major order), and `events`. Expected state is literal JSON;
no expected state is computed from the owner implementation.

The workspace audit's injected `set_turbo` with `on: false` is accepted without
an owner clock. `on: true` is rejected: these vectors do not model scheduler time.

Messages: power ID `0x8A07AE00` uses **normalized block bounds** `blocks` (AC5,
native decoding is unimplemented); budget ID `0x6A5FD3FD` uses `occupant`;
`0x231BBF91` refreshes every block; translation `0xA6B79612` reads the supplied
world view; zoom/rotation `0xA6B79602` uses `zoom`, `old_rotation`, `new_rotation`.
Other IDs exercise the unknown-message case.

Primitive event strings record acquired/released leases, occupant/rectangle/byte
reads, power tests, target add/remove (including raw `0xC0`) and flag changes.
These are fixture observations, not original COM reference-count measurements.
Failure, reference, ordering and edge rules absent from the spec are explicitly
marked AC5–AC8 choices in vector descriptions. AC2 scenarios prove refusal to
guess the budget predicate; they do not prove budget compatibility.
