# renderer-driver: open questions
<!-- Questions about [renderer-driver.md](../renderer-driver.md). Append new entries at the END; when one is answered, move it to answered/renderer-driver.md. Rules: README.md. -->

<!-- from: re-driver4 (2026-10-21) -->
- **RD1 - section 4.2:** which callers pass non-zero fourth arguments to `SetVideoMode`, and how do they handle
  its dual use? The low byte requests a flip chain (§14.2), but setup failure writes `0xFFFFFFFF` through the
  full argument word (§4.2, `0x0088916E`–`0x0088917F`). Resolve caller intent and whether the faulting small-boolean
  failure path is exercised. [needs: indirect callers and setup failure observations]
- **RD3 - section 6:** the unknown mode-record fields (+0x10..+0x1F, the "has stencil" byte +0x21 and who sets it,
  byte +0x22, the 14th stored dword copied from object `+0x274`), whether `0x00887D80` is a plain append, and the
  contracts of `SetViewport` (74 with four arguments, 75 with none) and `GetViewport` (76).
  *Partly answered 2026-10-21 (re-driverdraw4): slots 74-76 are §11 (open there: the writer of byte `+0x261`);
  the mode-record fields are still open.*
  *Further narrowed 2026-10-23 (re-gdriver5): §14.4 specifies mask fields, private selector dword and capability
  writes, and identifies append as vector insertion. Still open: full meanings of descriptor capability bytes,
  especially +0x22 and +0x2C, and every format-enumeration outcome.*
- **RD4 - section 4.4:** what `0x00887260` does at the frame boundary (end scene and present? windowed blit to the
  client rectangle?), and who handles message `0xAACB92A3` (device lost), which is not in `re/class_id_table.tsv`.
  *Partly answered 2026-10-21 (re-driverdraw4): `0x00887260` only ends the scene (§11.1); presentation is
  inferred to be the context's per-frame update.*
  *Further narrowed 2026-10-23 (re-gdriver5): §14.3 establishes Flip versus Blt presentation, empty-client
  result and retry flags. Still open: the `0xAACB92A3` handler.*

<!-- from: re-gdriver5 (2026-10-23) -->
- **RD6 - section 9.6 (remaining arithmetic only):** instruction-check the pixel arithmetic of available
  non-copy converters other than source 5 → keys 1/2 and source 9 → keys 1/2: the remaining uncompressed
  channel pack/quantization/expansion cells, and DXT1/source 10 or DXT3/source 12 → ordinary keys 1/2, 4/5, 7,
  including endpoint interpolation, alpha and partial-block/stride handling. Source 0 → keys 1/2 is an
  established bitwise copy. Ordinary ranking, all 120 descriptor-dependent availability cells, compressed
  preference/fallback, direct compressed byte copying and the two named expansions are answered and archived
  (2026-10-23, re-gdriver-rd11); do not reopen them. [needs: converter bodies selected by 0088F310–0088F737,
  0088D210/0088D270, with shared decode helpers 0080A6A0/0080A6D0]
- **RD7 - section 12.5 (CopyColorBuffer only):** establish the behavior of owner+4 slot 3 (`0x00883950`):
  lock-probe cache lifetime, destination allocation/refcount ownership, clipping/orientation, pixel-format
  conversion and every failure return. Table/identity attribution, all light/material operations, exact
  D3DLight range, constant/no-op slots, absence of independently returned vertex buffers in these tables,
  and region handles/transfers/recovery are answered and archived (2026-10-23, re-gdriver-rd11).
  [needs: instruction checks of 00883950–00883EB7; decompiler has unreliable pointer/stack aliases]
