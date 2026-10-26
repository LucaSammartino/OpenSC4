# sound-system: answered questions

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
