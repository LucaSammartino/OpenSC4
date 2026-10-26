# sound-system: answered questions

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
