# sound-system: answered questions

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS9 — Manager creation/cache and complete failure paths:** [unsettled]

For §7.2, recover instruction-confirmed construction defaults, GetSound and
GetSndDataSource resource/creator/cache resolution, autoplay ownership and
shutdown behavior. Most exported pseudocode is unavailable because of a
structure error; use instructions at 0x00969AF1, 0x009693C0, 0x00969055,
0x009694A2, 0x0096966C. Existing audio AU9 concerns the sound-ID tag and is
not answered or duplicated by this item.

**ANSWERED 2026-10-26 (re-ss29c):** DIFFERS — §7.2 gives instruction-confirmed
defaults, separate GetResource delegation, renderer-gated sound lookup,
registered-object QI without creator fallback, creator ordering, output/failure
boundaries, autoplay retention before Play and one-head-per-update cleanup.
Shutdown preserves registered data sources and service references until
unregistration/destruction; it does not clear every registry. Worked lookup
and ownership cases are included. AU9 is neither changed nor duplicated.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS10 — Director complete inherited slots and cache setup:** [unsettled]

For §8, enumerate inherited lifecycle slots, factory return/aggregation failure
contracts, and the sound-director PostAppInit cache operation (0x00966678).
The observed constant 0xCC3014EE does not establish the cache operation's meaning.

**ANSWERED 2026-10-26 (re-ss29c):** DIFFERS — §§8.1–8.3 enumerate all 17
primary and 11 secondary slots, including folded true no-ops, backend shutdown
callback and reference behavior. Kind-1 no-argument factories return identities
for QI; no controlling-outer/aggregation argument exists, unsupported queries
are Boolean failures, and null allocation results are unchecked by the shared
caller. PostAppInit creates the global property set through cache slot 3,
ignores its Boolean result, and dereferences a null cache if the service exists.
Factory/partial-start/cache worked cases are included.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS11 — Playlist complete IO and format compatibility:** [unsettled]

For §9, settle constructor/Open clearing versus appending, file failure results,
title/duration fields, ordinary M3U comments/URLs, PLS ordering/count/version,
UNC/dot/parent path normalization and string conversion. Start at 0x008D6B22,
0x008D6824, 0x008D66C9, 0x008D6196. No COM/drawing/message API has been
established for this parser.

**ANSWERED 2026-10-26 (re-ss29c):** DIFFERS — §§9.1–9.4 specify append-only
Open, IO false returns and partial-state retention, line trimming/conversion,
unconsumed unterminated final text, title/duration fields, plain comments/URLs
as records, sequential PLS lookahead/count/version handling, and literal
backslash/UNC/drive/dot processing with worked cases. No playlist COM API is
established. Narrowed runtime observations remain explicit in §9.1: record
locale/code-page-dependent non-ASCII conversion bytes; observe zero-size-read
and injected allocation-failure outcomes before the unchecked scan/read path.
The conversion call/status policy and unsafe boundaries are statically fixed;
no guessed locale-independent replacement table or empty-file success is given.

<!-- from: core-sound5 (2026-10-23) -->
- **SS12 — Deferred list duplicates and drain order:** [literal lists]

For §7.1, when the same handler or target is registered repeatedly, does
unregistration remove one matching entry or every matching pointer? For target
changes deferred during enumeration, are adds/removes drained in request order,
or in separate batches like the explicitly specified handler queues? The core
currently retains each registration, removes every matching pointer, and drains
target changes in request order; handlers drain registrations before removals.
For §9's otherwise ordinary relative paths, is choosing the base directory's
slash style and joining with one separator correct? SS11 already reserves exact
separator/UNC/dot behavior; this records the literal simple-path choice used by
the parser continuation, not a claim that Open is settled.

**ANSWERED 2026-10-26 (re-ss29c):** DIFFERS — §7.1: immediate registration
unregisters before reinsertion; unregistration erases the first match per list.
Targets drain removal, fast-add, slow-add batches; handler adds replay LIFO,
then removals FIFO. §9: segments are concatenated literally with backslash
delimiting, without choosing a slash style or inserting a separator. Worked
cases and instruction citations are in those sections.
