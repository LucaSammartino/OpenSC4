# sound-system: open questions

## SS1 — HIT instruction and register execution

In sound-system.md §3, settle all DoCommand opcodes, operand widths, branches,
timer/tempo arithmetic, complete register notifications and Play argument modes.
Starting points: 0x0093C229, 0x0093DF4C, 0x0093BD06 and 0x0093E146.
The export has overlapping/incorrect structures; instruction verification is required.

## SS2 — HIT manager resource arbitration and file grammar

For sound-system.md §4.1, settle HIT marker layout and entry offsets, duplicate
track/semantic registrations, group/duck priorities, pause/unpause slots and
source-handler payloads. Start with 0x00938D8E, 0x00937C00, 0x00938BF1,
0x00938AF0 and the table at 0x00AD2968. Constructor priority and seed are
export facts, but exact random range/distribution is not yet instruction-verified.

## SS3 — Channel tables and property transformations

For §4.2, enumerate unnamed tables installed by 0x0093FE12 (primary 0x00AD3FC0)
and settle QI views, notification dispatch, pitch/volume/pan formulas and patch
failure recovery (0x0093F07E, 0x0093F136, 0x0093FA5B, 0x0093F687).
The export's named-vtable query returns no cHitChannel table.

## SS4 — HIT INI complete initialization grammar

For §5.1, settle every section's token grammar and initialization order, numeric
conversion failure, duplicate keys and aggregate success on partial failure.
Start at EnumDefaultEntries 0x0093756A and the helper inventory in the spec.

## SS5 — Track resource byte/text formats and private copies

For §5.2, settle TKDT legacy field order, escaping/version handling, serializer
format, read failure, assignment/private-copy ownership and which constructor
installs the alternate tables. Start at 0x00940E5C, 0x00941930, 0x00939D62,
0x00939FFC. Do not implement a decoder from the TKDT prefix alone.

## SS6 — Adaptive score selection and full lifecycle

For §6.1, settle Init overload signatures, dimension limits, cell add/subtract
selection, probabilities, random reseeding, coordinate conversion, tempo,
volume fade law and stop/shutdown order. Start at 0x00947220, 0x00948719,
0x00947E90, 0x00947D28 and 0x0094832A. The probability-named function
0x009477A6 actually counts busy cells; method names are not sufficient evidence.

## SS7 — Cell start, spatial policy and external controller

For §6.2, settle Start/StartCellExternal, AutoPan, special loop/countdown state,
filename GUID recognition, external-controller notification and all failure
paths. Start at 0x00946C2C, 0x0094680F, 0x00945AAF, 0x009459BF.

## SS8 — Sound-system timing, listener and lifecycle

For §7.1, settle RegisterTarget period classification/slow timer boundary,
extension map operations and tail/folded slots, listener numeric conversions,
renderer duplicate rules, cache ownership, initialization dependencies and
shutdown queue/reference order. Start at 0x00967DD4, 0x00967885,
0x00967FE4, 0x009683EF. Preserve the confirmed consuming event chain.

## SS9 — Manager creation/cache and complete failure paths

For §7.2, recover instruction-confirmed construction defaults, GetSound and
GetSndDataSource resource/creator/cache resolution, autoplay ownership and
shutdown behavior. Most exported pseudocode is unavailable because of a
structure error; use instructions at 0x00969AF1, 0x009693C0, 0x00969055,
0x009694A2, 0x0096966C. Existing audio AU9 concerns the sound-ID tag and is
not answered or duplicated by this item.

## SS10 — Director complete inherited slots and cache setup

For §8, enumerate inherited lifecycle slots, factory return/aggregation failure
contracts, and the sound-director PostAppInit cache operation (0x00966678).
The observed constant 0xCC3014EE does not establish the cache operation's meaning.

## SS11 — Playlist complete IO and format compatibility

For §9, settle constructor/Open clearing versus appending, file failure results,
title/duration fields, ordinary M3U comments/URLs, PLS ordering/count/version,
UNC/dot/parent path normalization and string conversion. Start at 0x008D6B22,
0x008D6824, 0x008D66C9, 0x008D6196. No COM/drawing/message API has been
established for this parser.
