# sound-system: open questions
<!-- Engine questions and RE gaps for docs/specs/sound-system.md. -->

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS5 — Track resource byte/text formats and private copies:** [unsettled]

For §5.2, settle TKDT legacy field order, escaping/version handling, serializer
format, read failure, assignment/private-copy ownership and which constructor
installs the alternate tables. Start at 0x00940E5C, 0x00941930, 0x00939D62,
0x00939FFC. Do not implement a decoder from the TKDT prefix alone.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS6 — Adaptive score selection and full lifecycle:** [unsettled]

For §6.1, settle Init overload signatures, dimension limits, cell add/subtract
selection, probabilities, random reseeding, coordinate conversion, tempo,
volume fade law and stop/shutdown order. Start at 0x00947220, 0x00948719,
0x00947E90, 0x00947D28 and 0x0094832A. The probability-named function
0x009477A6 actually counts busy cells; method names are not sufficient evidence.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS7 — Cell start, spatial policy and external controller:** [unsettled]

For §6.2, settle Start/StartCellExternal, AutoPan, special loop/countdown state,
filename GUID recognition, external-controller notification and all failure
paths. Start at 0x00946C2C, 0x0094680F, 0x00945AAF, 0x009459BF.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS8 — Sound-system timing, listener and lifecycle:** [unsettled]

For §7.1, settle RegisterTarget period classification/slow timer boundary,
extension map operations and tail/folded slots, listener numeric conversions,
renderer duplicate rules, cache ownership, initialization dependencies and
shutdown queue/reference order. Start at 0x00967DD4, 0x00967885,
0x00967FE4, 0x009683EF. Preserve the confirmed consuming event chain.
- **SS1 — HIT instruction and register execution:** [unsettled]

In sound-system.md §3, settle all DoCommand opcodes, operand widths, branches,
timer/tempo arithmetic, complete register notifications and Play argument modes.
Starting points: 0x0093C229, 0x0093DF4C, 0x0093BD06 and 0x0093E146.
The export has overlapping/incorrect structures; instruction verification is required.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS2 — HIT manager resource arbitration and file grammar:** [unsettled]

For sound-system.md §4.1, settle HIT marker layout and entry offsets, duplicate
track/semantic registrations, group/duck priorities, pause/unpause slots and
source-handler payloads. Start with 0x00938D8E, 0x00937C00, 0x00938BF1,
0x00938AF0 and the table at 0x00AD2968. Constructor priority and seed are
export facts, but exact random range/distribution is not yet instruction-verified.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS3 — Channel tables and property transformations:** [unsettled]

For §4.2, enumerate unnamed tables installed by 0x0093FE12 (primary 0x00AD3FC0)
and settle QI views, notification dispatch, pitch/volume/pan formulas and patch
failure recovery (0x0093F07E, 0x0093F136, 0x0093FA5B, 0x0093F687).
The export's named-vtable query returns no cHitChannel table.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS4 — HIT INI complete initialization grammar:** [unsettled]

For §5.1, settle every section's token grammar and initialization order, numeric
conversion failure, duplicate keys and aggregate success on partial failure.
Start at EnumDefaultEntries 0x0093756A and the helper inventory in the spec.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS9 — Manager creation/cache and complete failure paths:** [unsettled]

For §7.2, recover instruction-confirmed construction defaults, GetSound and
GetSndDataSource resource/creator/cache resolution, autoplay ownership and
shutdown behavior. Most exported pseudocode is unavailable because of a
structure error; use instructions at 0x00969AF1, 0x009693C0, 0x00969055,
0x009694A2, 0x0096966C. Existing audio AU9 concerns the sound-ID tag and is
not answered or duplicated by this item.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS10 — Director complete inherited slots and cache setup:** [unsettled]

For §8, enumerate inherited lifecycle slots, factory return/aggregation failure
contracts, and the sound-director PostAppInit cache operation (0x00966678).
The observed constant 0xCC3014EE does not establish the cache operation's meaning.

<!-- from: re-sound-hit5 (2026-10-23) -->
- **SS11 — Playlist complete IO and format compatibility:** [unsettled]

For §9, settle constructor/Open clearing versus appending, file failure results,
title/duration fields, ordinary M3U comments/URLs, PLS ordering/count/version,
UNC/dot/parent path normalization and string conversion. Start at 0x008D6B22,
0x008D6824, 0x008D66C9, 0x008D6196. No COM/drawing/message API has been
established for this parser.

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
