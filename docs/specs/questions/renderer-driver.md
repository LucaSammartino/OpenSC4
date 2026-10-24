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

