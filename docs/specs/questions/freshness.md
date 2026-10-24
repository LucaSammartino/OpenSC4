# freshness: open questions
<!-- Questions about [freshness.md](../freshness.md). Append new entries at the END; when one is answered, move it to answered/freshness.md. Rules: README.md. -->

<!-- from: re-freshness (2026-10-22) -->
- **FS1 - section 3.1:** the roles of cell fields 8 (`+0x3C`), 9 (`+0x44`), 12 (`+0x4C`), 16 (`+0x20`) and 18
  (`+0x60`), and of score `+0x38`/`+0x3C`: parsed, with no reader in the `cFresh*` classes. [unused by this build]

<!-- from: re-freshness (2026-10-22) -->
- **FS2 - section 4.6 step 3:** what sets the start switch `+0x4088` and the two suspend bytes `+0x4090`/`+0x4091`
  the beat clock tests; no `cFreshPlayer` method read here writes them. [set through the +0x04 interface]

<!-- from: re-freshness (2026-10-22) -->
- **FS3 - section 5.3:** the looping cell's stop rule while the stopping mark is set (the export stops while the
  loop counter is non-zero, which reads inverted). [stop when the counter reaches 0]

<!-- from: re-freshness (2026-10-22) -->
- **FS4 - section 4.9:** the meaning of the callback timer's slot-1 call with 100 before it is started. [a priority
  or resolution]

<!-- from: re-freshness (2026-10-22) -->
- **FS5 - section 2.2:** confirm `cRZString::SplitToken`'s not-found behaviour (empty result, string unchanged),
  which the score, Hit-list and TKDT readers all depend on. [as stated]
