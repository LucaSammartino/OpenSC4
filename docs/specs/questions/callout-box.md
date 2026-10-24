# callout-box: open questions
<!-- Questions about [callout-box.md](../callout-box.md). Append new entries at the END; when one is answered, move it to answered/callout-box.md. Rules: README.md. -->

<!-- from: re-callout7 (2026-10-23) -->
- **WCB1 - §3.1:** the font slots used to lay out and draw the texts: 45 `(text, length, &rect, 0, 250, maxW)`
  (measure in a box), 55 and 56 (begin and end drawing on a surface) and 52 `(surface, &rect, text, length, 0,
  colour, 0x8000000, 0, 0)`. What are the 250 and the `0x8000000` flag (word wrap)? [inferred]

<!-- from: re-callout7 (2026-10-23) -->
- **WCB2 - §3.4:** the drawing offset is minus the parent's slot-50 point, while the callout window is usually
  given its parent's whole area. Is slot 50 the window's position in its parent (so the box is drawn where the
  parent-relative rectangles say)? [inferred]

<!-- from: re-callout7 (2026-10-23) -->
- **WCB3 - §3.1:** the text surface is locked with `0x8010` before the text is drawn and is not unlocked in this
  function. Do the font slots 55/56 release that lock, or does the surface stay locked until its next use?
  [unknown]
