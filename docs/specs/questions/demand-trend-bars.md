# demand-trend-bars: open questions
<!-- Questions about [demand-trend-bars.md](../demand-trend-bars.md). Append new entries at the END; when one is answered, move it to answered/demand-trend-bars.md. Rules: README.md. -->

<!-- from: re-gzbars7 (2026-10-24) -->
- **RCI1 - §1.4:** the horizontal bar for positive demand passes its fill rectangle with left > right (left =
  half + 1 + x, right = half + 1). Does the window draw context's slot 35 normalise the rectangle (so the bar is drawn
  to the right of the centre), or does it draw nothing (so horizontal RCI bars never show positive demand)? The game's
  RCI meter is vertical, so this may never show. [static: the context's slot 35 body; dynamic: a horizontal bar]
