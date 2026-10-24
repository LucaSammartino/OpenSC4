# widget-wingen: open questions
<!-- Questions about [widget-wingen.md](../widget-wingen.md). Append new entries at the END; when one is answered, move it to answered/widget-wingen.md. Rules: README.md. -->

<!-- from: re-gzwingen7 (2026-10-23) -->
- **WGN1 - §5.3:** background blit types 0 and 1 use draw-context slots 39 `(image, src, dest, 0, 0)` and 38
  `(image, src, dest)`. The spec reads 39 as a tiled fill with zero offsets and 38 as the plain copy; the 2D draw
  context is not specified yet. [inferred]

<!-- from: re-gzwingen7 (2026-10-23) -->
- **WGN2 - §8.1, §9.2:** the window-control manager's `GetColor` (slot 32) result is passed whole to window slot
  106 but cut to 16 bits for flat-rect slot 8, which turns it into `r, g, b` through graphics-system slot 36. Is the
  stored colour a 16-bit native (display-format) value, and are graphics-system slots 34/36 the RGB ↔ native
  conversions? [inferred]

<!-- from: re-gzwingen7 (2026-10-23) -->
- **WGN3 - §7:** `GZOnCommand` closes the window for button IDs `0x42B7C353` and `0x42B7C356`. The title bar creates
  `0x42B7C353`, `0x42B7C354` and `0x42B7C355` (close, min/max and go-back by their flag bits); which control sends
  `0x42B7C356`? [inferred]
