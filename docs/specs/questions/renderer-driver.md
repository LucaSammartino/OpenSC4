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
- **RD6 - section 9.6:** complete the ordinary internal-format ranking of `0x0088F8D0`, the 15×8 converter
  availability matrix and per-converter rounding, bit expansion and compressed-block handling. Compressed-key
  preference and fallback requests are specified; full arithmetic is not. [needs: instruction-checked format
  selector and converter families; distinguish texture descriptor enumeration from surface allocation]
- **RD7 - sections 12.1 and 14.1:** complete the auxiliary interfaces at owner +0x0/+0x4/+0x8: individual light
  enable/property calls, material operations, independently exposed vertex buffers and buffer regions. Establish
  exact range constant evaluation for D3DLight. These are outside the 83 graphics slots. [needs: per-table
  dispatch attribution; instruction checks of numeric setters; public call/return ownership]
- **RD8 - section 14.2:** complete context device-selector choices (-1/-3/-4 and concrete values), depth-format
  enumeration/ranking and every CreateDevice/depth-attachment failure path in `0x0088BFC0`. Current spec covers
  cooperative flags, flip-count fallback, separate surfaces, clipper and presentation. [needs: callbacks
  `0x0088BDE0`, `0x0088BE40`, `0x0088BDC0` and all remaining setup instruction pages]

<!-- from: core-gdriver5 (2026-10-23) -->
- **RD9 - sections 2, 4.1, 4.4 and 10.1:** specify the complete texture-stage constructor/default-reset values,
  initial material/clear state, the Shutdown error helper, adapter-info storage and zero-version fallback,
  and driver/manager final-release destruction behavior. Section 4.4 refers to 5.4 for stage defaults, but
  that section only specifies capability setters. [current: explicit Unsupported boundary for unknown
  reset/helper/info operations; test defaults are supplied fixtures, not native claims]
- **RD10 - section 8.3:** when compacting position-only, position+colour or position+UV into the format-11
  buffer, what values are written to missing colour/UV elements? What are the exact generated index count
  and buffer contents for incomplete quads and the result selected when several queued submissions fail?
  [current: copy only specified source elements, leave missing destination bytes untouched; complete quads
  only; retain the last failing submission HRESULT; these choices require confirmation]
- **RD11 - section 8.6:** must portable core match x87 projection conversion bit-for-bit for all possible
  float matrices/viewport sizes, including non-finite inputs and double-rounding cases, or are ordinary
  finite conformance vectors sufficient? [current: f64 intermediates with single output rounding; exact
  x87 quotient/extended accumulator emulation is pending clarification]
