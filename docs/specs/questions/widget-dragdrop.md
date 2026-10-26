# widget-dragdrop: open questions
<!-- Questions about [widget-dragdrop.md](../widget-dragdrop.md). Append new entries at the END; when one is answered, move it to answered/widget-dragdrop.md. Rules: README.md. -->

<!-- from: re-gzdragdrop7 (2026-10-24) -->
- **WDD1 - §4:** the pending drag starts when the pointer (in `W`'s coordinates) and the same point converted into
  the source's coordinates (window slot 61) differ by more than 3 in x or y; the grab point `D+0x1C/+0x20` is not
  read. Unless `W` lies exactly over the source this trips on the first move. Does any shipped window register as a
  drop source proxy (callers of slots 12 and 13), and does a drag start at once in game? [game: press and move one
  pixel on such a window]
