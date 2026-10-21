# renderer-driver: open questions
<!-- Questions about [renderer-driver.md](../renderer-driver.md). Append new entries at the END; when one is answered, move it to answered/renderer-driver.md. Rules: README.md. -->

<!-- from: re-driver4 (2026-10-21) -->
- **RD1 - section 4.2:** what does the fourth argument of `SetVideoMode` (slot 66, stored as a byte at object
  `+0x25F`) control, and which callers pass non-zero? [needs: readers of that byte in `nSGLDX7::cGDriver`]
- **RD3 - section 6:** the unknown mode-record fields (+0x10..+0x1F, the "has stencil" byte +0x21 and who sets it,
  byte +0x22, the 14th stored dword copied from object `+0x274`), whether `0x00887D80` is a plain append, and the
  contracts of `SetViewport` (74 with four arguments, 75 with none) and `GetViewport` (76).
- **RD4 - section 4.4:** what `0x00887260` does at the frame boundary (end scene and present? windowed blit to the
  client rectangle?), and who handles message `0xAACB92A3` (device lost), which is not in `re/class_id_table.tsv`.
- **RD5 - section 1:** the texture-name, texture-upload and draw slots (24-29, 35-39, 51-62, 3-5, 68-73) and the
  `cTextureManager` / `cPrimitiveManager` contracts behind them (queueing, batch limits, when a queued batch is
  submitted besides state changes and `Flush`).
  *Partly answered 2026-10-21 (re-driverdraw4): `DrawArrays` (slot 3), `InterleavedArrays` (5) and the whole
  `cPrimitiveManager` queue / batch / flush contract are now §8.2-§8.3. `DrawElements` (4) is §8.2 too. Still open:
  slots 6-10, 24-29, 35-42, 51-62, 68-76, the per-stage record contents (`0x00884320`, `0x00884500`) and
  `cTextureManager` (`0x0088A380`-`0x0088B2C0`).*
