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
