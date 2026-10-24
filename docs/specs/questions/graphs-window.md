# graphs-window: open questions
<!-- Questions about [graphs-window.md](../graphs-window.md). Append new entries at the END; when one is answered, move it to answered/graphs-window.md. Rules: README.md. -->

<!-- from: re-graphs7 (2026-10-23) -->
- **GW1 - §4, §5, §6:** what the graph widget slots do (`G4`…`G51`, `B5`…`B21`, `L5`…`L20`, and `G43`, which
  `cSC4LineGraph` overrides with an antialiased line). They are named by number only until `cGZGraph`,
  `cGZLineGraph` and `cGZBarGraph` are specified. [unknown]

<!-- from: re-graphs7 (2026-10-23) -->
- **GW2 - §5.2 step 5:** the second argument of the two `G17` calls, 0 and the number of points `N`. Is it the
  sample index at which the axis title is drawn (the left and right ends)? [inferred]

<!-- from: re-graphs7 (2026-10-23) -->
- **GW3 - §3:** `SelectOption` calls slot 5 `(1, 0)` and `(1, 1)` on the buttons `0x4000 + g` and `0x4100 + g`.
  Is that the toggle state (off, then on)? [inferred]

<!-- from: re-graphs7 (2026-10-23) -->
- **GW4 - §4 step 6:** what the line-only control `0x8A2871D8` is (shown with the time slider `0xEA4A7CF2` and its
  label `0x8A2871D9`). [unknown]
