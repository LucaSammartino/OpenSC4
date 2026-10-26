# rz-draw: open questions
<!-- Questions about [rz-draw.md](../rz-draw.md). Append new entries at the END; when one is answered, move it to answered/rz-draw.md. Rules: README.md. -->

<!-- from: re-rzdraw7 (2026-10-24) -->
- **RZD1 - §3:** the exact pixel sets of `RZDrawDrawCircle` for thickness 2 ≤ t < r (how the rows and columns between
  the outer and inner midpoint circles are filled per octant, and whether the two circles' points themselves are
  included) and of `DrawLineAntialiased` for slopes other than 0, 1 and ∞ (plain stepping or Wu-style weighting).
  [static: `0x008D6CF0`–`0x008D8749` and `0x00985690`; dynamic: draw a few rings and lines into a known buffer]
