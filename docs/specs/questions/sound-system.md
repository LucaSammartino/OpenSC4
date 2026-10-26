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
