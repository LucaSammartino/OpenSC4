# minimap: open questions
<!-- Questions about [minimap.md](../minimap.md). Append new entries at the END; when one is answered, move it to answered/minimap.md. Rules: README.md. -->

<!-- from: re-minimap7 (2026-10-23) -->
- **MMW1 - §5.2:** what terrain slot 16 returns per cell (the water value used directly as a water-palette index):
  water depth in metres below the sea level, or something else? [inferred: depth]

<!-- from: re-minimap7 (2026-10-23) -->
- **MMW2 - §2, §8:** the window fields the paint uses: `+0x24`/`+0x28` (the blit origin and the frustum offset),
  `+0x34`…`+0x40` (the 4444 copy area) and `+0xA8`…`+0xB4` (the marker scale). Are these the window's area in its
  parent and its client rectangle? [inferred]

<!-- from: re-minimap7 (2026-10-23) -->
- **MMW3 - §8.3, §10.1.1:** which object `city` slot 49 and its slot 9 return (the blinking marker's subject), and
  what its slot 6 tests on the tick. [unknown]
