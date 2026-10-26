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
- **SS1 — HIT instruction and register execution:**

In sound-system.md §3, settle all DoCommand opcodes, operand widths, branches,
timer/tempo arithmetic, complete register notifications and Play argument modes.
Starting points: 0x0093C229, 0x0093DF4C, 0x0093BD06 and 0x0093E146.
The export has overlapping/incorrect structures; instruction verification is required.

**ANSWERED 2026-10-26 (re-ss29a):** DIFFERS — §3 specifies Play modes,
notification dispatch, verified initial opcode widths/branches, illegal/no-op
sets and timer/ramp pacing, including the shared timer tail misnamed NoteOff.
Complete interpreter closure is **narrowed**, not claimed: §3.3 lists the exact
residual opcode set and concrete operand/PC/flag/interface/error/tempo observations.
No runtime trace was performed. Worked cases cover unsigned byte loads, word
order, discarded wait excess and strict ramp arrival.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS2 — HIT manager resource arbitration and file grammar:**

For sound-system.md §4.1, settle HIT marker layout and entry offsets, duplicate
track/semantic registrations, group/duck priorities, pause/unpause slots and
source-handler payloads. Start with 0x00938D8E, 0x00937C00, 0x00938BF1,
0x00938AF0 and the table at 0x00AD2968. Constructor priority and seed are
export facts, but exact random range/distribution is not yet instruction-verified.

**ANSWERED 2026-10-26 (re-ss29a):** DIFFERS — §4.1 specifies whole-file ENTP
offsets, unused TRAX search, duplicate replacement/newest-block lookup, semantic
ID-zero insertion, duck signed maximum, actual pause slots and exact random law.
Source calls are an unchanged tail-forward to retained handler slot 3. Semantic
source payload and control-object priority policy are narrowed to explicit
recording-handler/runtime observations; manager behavior has worked cases.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS3 — Channel tables and property transformations:**

For §4.2, enumerate unnamed tables installed by 0x0093FE12 (primary 0x00AD3FC0)
and settle QI views, notification dispatch, pitch/volume/pan formulas and patch
failure recovery (0x0093F07E, 0x0093F136, 0x0093FA5B, 0x0093F687).
The export's named-vtable query returns no cHitChannel table.

**ANSWERED 2026-10-26 (re-ss29a):** DIFFERS — §4.2 enumerates all six tables,
QI offsets, property dispatch and instruction-verified signed volume/pitch laws.
It records first-use pitch caching, inverted pitch result and unchecked patch
creation failure, with worked cases. No original rollback/recovery is promised.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS4 — HIT INI complete initialization grammar:**

For §5.1, settle every section's token grammar and initialization order, numeric
conversion failure, duplicate keys and aggregate success on partial failure.
Start at EnumDefaultEntries 0x0093756A and the helper inventory in the spec.

**ANSWERED 2026-10-26 (re-ss29a):** DIFFERS — §5.1 now specifies all initialization
stages, numeric-map replacement and ordering, delimiters, unchecked unsigned
conversion, creator callback payloads, exclusion sets and nonaggregating success.
INI interface is at +0x3C, not +0x4C. Missing HIT/creator dependencies are not
uniformly checked. Runtime observations are stated for inherited repeated-section
and locale behavior; well-formed HIT initialization is statically specified.
- **SS5 — Track resource byte/text formats and private copies:**

For §5.2, settle TKDT legacy field order, escaping/version handling, serializer
format, read failure, assignment/private-copy ownership and which constructor
installs the alternate tables. Start at 0x00940E5C, 0x00941930, 0x00939D62,
0x00939FFC. Do not implement a decoder from the TKDT prefix alone.

**ANSWERED 2026-10-26 (re-ss29b):** DIFFERS — sound-system.md §5.2 now gives
both constructor/table pairs, serial-record framing, all 27 ordered fields,
version-dependent omissions, literal-comma parsing, the sem default, checked
self-reference rejection, XML output selection and copy/reference ownership.
Legacy invalid-record reads have no checked failure recovery; §5.2.1 states
the mock-record observations needed to establish their return/crash behavior.
Worked cases cover version 0, self-reference failure, signed decimal output,
ignored write failure and scalar-copy independence.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS6 — Adaptive score selection and full lifecycle:**

For §6.1, settle Init overload signatures, dimension limits, cell add/subtract
selection, probabilities, random reseeding, coordinate conversion, tempo,
volume fade law and stop/shutdown order. Start at 0x00947220, 0x00948719,
0x00947E90, 0x00947D28 and 0x0094832A. The probability-named function
0x009477A6 actually counts busy cells; method names are not sufficient evidence.

**ANSWERED 2026-10-26 (re-ss29b):** DIFFERS — sound-system.md §§6.1.1–6.1.3
give all five Init contracts, unchecked dimension/storage preconditions,
selection traversal and tie rules, integer start scores, separate random
sources, coordinate/tempo divisions, quantized fades and destruction order.
Instructions establish class 0 returns zero and group-zero class 20 scores
100, correcting the older freshness spec's caveats. Worked cases cover
coordinates, ties, probability boundaries and fade overshoot.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS7 — Cell start, spatial policy and external controller:**

For §6.2, settle Start/StartCellExternal, AutoPan, special loop/countdown state,
filename GUID recognition, external-controller notification and all failure
paths. Start at 0x00946C2C, 0x0094680F, 0x00945AAF, 0x009459BF.

**ANSWERED 2026-10-26 (re-ss29b):** DIFFERS — sound-system.md §§6.2.1–6.2.2
specify admission, filename detection versus extraction, sound/QI failures,
spatial conversions, separate AutoPan, signed pitch remainder, external
controller arguments and notification order. A refused external start leaves
state 2; marked looping kills on nonzero decremented count. Worked cases
cover those state transitions, pan and filename recognition. Required cell,
manager and 3D views are explicit unchecked preconditions.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS8 — Sound-system timing, listener and lifecycle:**

For §7.1, settle RegisterTarget period classification/slow timer boundary,
extension map operations and tail/folded slots, listener numeric conversions,
renderer duplicate rules, cache ownership, initialization dependencies and
shutdown queue/reference order. Start at 0x00967DD4, 0x00967885,
0x00967FE4, 0x009683EF. Preserve the confirmed consuming event chain.

**ANSWERED 2026-10-26 (re-ss29b):** DIFFERS — sound-system.md §§7.1.1–7.1.3
give the exact period-20 classifier, unsigned >250-ms boundary and deferred
queue order; normalized extension map and seed-dependent extension-text
enumeration; completed primary slots; f32 listener forwarding; renderer
identity and cache reference contracts; Init dependencies and unconditional
success; and service-relative teardown offsets/order. The consuming event
chain is preserved. Worked cases cover timing, queues, map quirks, listener
values, duplicate renderers and disabled Init; backend-dependent failure
outcomes have a precise runtime observation.
