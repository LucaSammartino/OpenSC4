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
