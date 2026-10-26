# sound-system: answered questions

<!-- from: re-sound-hit5 (2026-10-23) -->
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
