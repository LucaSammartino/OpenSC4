# renderer-driver: answered questions
<!-- Answered entries moved from ../renderer-driver.md. Rules: ../README.md. -->

<!-- from: re-gdriver5 (2026-10-23); settled portions, residual entries narrowed in open file -->
- **RD6 - section 9.6:** complete the ordinary internal-format ranking of `0x0088F8D0`, the 15×8 converter
  availability matrix and per-converter rounding, bit expansion and compressed-block handling. Compressed-key
  preference and fallback requests are specified; full arithmetic is not. [needs: instruction-checked format
  selector and converter families; distinguish texture descriptor enumeration from surface allocation]
  **ANSWERED 2026-10-23 (re-gdriver-rd11):** DIFFERS — §9.6 establishes total-bit-count/alpha ranking with
  stable enumeration ties, the descriptor-dependent availability rule for all 120 cells (including DXT5's
  lack of an ordinary decoder), exact direct compressed byte-copy handling, source-0 bitwise copy to keys
  1/2 and source-5/source-9 expansion to keys 1/2. DX7-39–44 discriminate. Full remaining converter arithmetic
  is not claimed: the open RD6 entry now names only the other non-copy cells and DXT1/DXT3 decoder details.
- **RD7 - sections 12.1 and 14.1:** complete the auxiliary interfaces at owner +0x0/+0x4/+0x8: individual light
  enable/property calls, material operations, independently exposed vertex buffers and buffer regions. Establish
  exact range constant evaluation for D3DLight. These are outside the 83 graphics slots. [needs: per-table
  dispatch attribution; instruction checks of numeric setters; public call/return ownership]
  **ANSWERED 2026-10-23 (re-gdriver-rd11):** DIFFERS — §§12.1, 12.4–12.5 attribute all three tables and
  specify light/material setters, raw low-byte enables, ambient bit conversion, sqrt(FLT_MAX) range,
  handle reuse/ownership, transfer rectangles and asymmetric recovery. No functional slot returns an
  independent vertex buffer; constant stubs are byte-true, not allocation. DX7-32–38 and DX7-45–49
  discriminate. Only owner+4 CopyColorBuffer's behavior beyond table attribution remains open under RD7;
  its decompiler pointer/stack aliases need separate instruction reconciliation.

<!-- from: re-gdriver5 (2026-10-23) -->
- **RD8 - section 14.2:** complete context device-selector choices (-1/-3/-4 and concrete values), depth-format
  enumeration/ranking and every CreateDevice/depth-attachment failure path in `0x0088BFC0`. Current spec covers
  cooperative flags, flip-count fallback, separate surfaces, clipper and presentation. [needs: callbacks
  `0x0088BDE0`, `0x0088BE40`, `0x0088BDC0` and all remaining setup instruction pages]
  **ANSWERED 2026-10-23 (re-gdriver-rd11):** DIFFERS — §14.2 specifies capability scoring for -1, literal
  GUID preference for -4, direct reference GUID for -3, undefined selection for other selectors, first-format
  depth enumeration without ranking, attachment cleanup and every checked setup failure. DX7-28–30 discriminate.

<!-- from: core-gdriver5 (2026-10-23) -->
- **RD9 - sections 2, 4.1, 4.4 and 10.1:** specify the complete texture-stage constructor/default-reset values,
  initial material/clear state, the Shutdown error helper, adapter-info storage and zero-version fallback,
  and driver/manager final-release destruction behavior. Section 4.4 refers to 5.4 for stage defaults, but
  that section only specifies capability setters. [current: explicit Unsupported boundary for unknown
  reset/helper/info operations; test defaults are supplied fixtures, not native claims]
  **ANSWERED 2026-10-23 (re-gdriver-rd11):** DIFFERS — §§2, 4.1 and 10.1 specify zero material,
  clear defaults, full stage constructor values and dirty-only reset. Shutdown's helper empties a vector,
  not the error channel; GetDriverInfo borrows the static string; zero versions use file-version lookup.
  Driver destruction AddRefs the texture manager while releasing the primitive manager. DX7-25–27/31.
- **RD10 - section 8.3:** when compacting position-only, position+colour or position+UV into the format-11
  buffer, what values are written to missing colour/UV elements? What are the exact generated index count
  and buffer contents for incomplete quads and the result selected when several queued submissions fail?
  [current: copy only specified source elements, leave missing destination bytes untouched; complete quads
  only; retain the last failing submission HRESULT; these choices require confirmation]
  **ANSWERED 2026-10-23 (re-gdriver-rd11):** DIFFERS — §8.3 writes opaque white missing colour and
  leaves missing UV untouched; incomplete nonindexed quads write whole groups beyond declared count,
  with indexed count-5/count-6 behavior distinguished. Flush returns the final submission HRESULT,
  even success following failure. DX7-19–23 give byte/count/result vectors.
- **RD11 - section 8.6:** must portable core match x87 projection conversion bit-for-bit for all possible
  float matrices/viewport sizes, including non-finite inputs and double-rounding cases, or are ordinary
  finite conformance vectors sufficient? [current: f64 intermediates with single output rounding; exact
  x87 quotient/extended accumulator emulation is pending clarification]
  *Implementation follow-up 2026-10-23:* a rational discriminator (width 30, matrix words `3e2f59ff`
  and `40a4645f`) proves f64 produces `30888888` versus x87 `30888889`. Core now uses a software
  64-significand-bit accumulator and correctly rounds finite operations; the remaining question is the
  FPU control-word assumptions and NaN-payload/exception behavior, which the spec does not prescribe.
  **ANSWERED 2026-10-23 (re-gdriver-rd11):** MATCHES — §8.6 requires finite extended-precision
  conformance under nearest/even with masked exceptions, including DX7-24's discriminator. The native
  function inherits the ambient x87 control word and does not canonicalize NaNs; portable core is not
  required to expose x87 traps/status/NaN payloads absent an explicit environment-compatibility mode.

<!-- from: re-driver4 (2026-10-21); closed by re-gdriver5 -->
- **RD5 - section 1:** the texture-name, texture-upload and draw slots (24-29, 35-39, 51-62, 3-5, 68-73) and the
  `cTextureManager` / `cPrimitiveManager` contracts behind them (queueing, batch limits, when a queued batch is
  submitted besides state changes and `Flush`). Subsequent scope included slots 6-10, 40-42 and 67-76,
  stage records and the writers of +0x209/+0x263.
  **ANSWERED 2026-10-23 (re-gdriver5):** MATCHES — §§8–11 and §13 specify these interface contracts:
  queued versus immediate geometry, batch limits, stage caching, name/surface lifetime, upload ordering,
  matrices and unsupported blits. §12.3 indexes all graphics slots. +0x209 comes from mode capability byte
  +0x29 (§10.2); +0x263 is setup/teardown readiness (§9.5). Complete format ranking/converter arithmetic
  is tracked separately as RD6, not asserted by this interface answer.

<!-- from: re-driver4 (2026-10-21) -->
- **RD2 - sections 4.1 and 4.3:** the contents of `SetDefaultD3DDeviceState` (`0x008872A0`, 788 bytes): which
  render and texture-stage states it sets and to what; and the purpose of the word at object `+0x138` that `Init`
  resets to 7. [needs: a pass over `0x008872A0` with `re_query.py asm`]
  **ANSWERED 2026-10-21 (re-driverdraw4):** MATCHES — section 7: 26 render states, 4 driver flags, 12 stage-0/1
  texture-stage calls and an unbound stage-0 texture in fixed order (stencil depth-fail defaults to invert,
  texture factor 0xFFFFFFFF, lighting off, colour vertex on), then every stage record marked dirty; the `+0x138`
  word has no reader in 0x00881A00-0x0088B400 (write-only).
