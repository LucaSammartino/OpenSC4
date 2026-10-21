# renderer-driver: open questions
<!-- Questions about [renderer-driver.md](../renderer-driver.md). Append new entries at the END; when one is answered, move it to answered/renderer-driver.md. Rules: README.md. -->

<!-- from: re-driver4 (2026-10-21) -->
- **RD1 - section 4.2:** what does the fourth argument of `SetVideoMode` (slot 66, stored as a byte at object
  `+0x25F`) control, and which callers pass non-zero? [needs: readers of that byte in `nSGLDX7::cGDriver`]
- **RD2 - sections 4.1 and 4.3:** the contents of `SetDefaultD3DDeviceState` (`0x008872A0`, 788 bytes): which
  render and texture-stage states it sets and to what; and the purpose of the word at object `+0x138` that `Init`
  resets to 7. [needs: a pass over `0x008872A0` with `re_query.py asm`]
- **RD3 - section 6:** contracts of `CountVideoModes` (63), `GetVideoModeInfo` (64 and 65: which record fields,
  in what order, and how full-screen / stencil are reported), `SetViewport` (74 with four arguments, 75 with none)
  and `GetViewport` (76).
- **RD4 - section 4.4:** what `0x00887260` does at the frame boundary (end scene and present? windowed blit to the
  client rectangle?), and who handles message `0xAACB92A3` (device lost), which is not in `re/class_id_table.tsv`.
- **RD5 - section 1:** the texture-name, texture-upload and draw slots (24-29, 35-39, 51-62, 3-5, 68-73) and the
  `cTextureManager` / `cPrimitiveManager` contracts behind them (queueing, batch limits, when a queued batch is
  submitted besides state changes and `Flush`).
