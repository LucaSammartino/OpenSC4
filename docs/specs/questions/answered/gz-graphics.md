# Answered GZ graphics questions

- **GZG1 — sections 2–3, generic geometry and convenience blits:** settle the
  signatures/geometry of context slots 36–37 (`0x0082C100`), kinds 2/4/5, line
  endpoint and polygon closure/coverage, gradient conversion, and exact
  Blt/BltTiled source rectangles and scaling. Basic filled result-1/result-2
  dispatch is specified in section 3.2; full tessellation coverage remains open.
  **Partial evidence 2026-10-23 (re-gzgraphics-gzg5):** §§3.2.1–3.2.2 now
  settle the folded four-integer ellipse calls, kind identities, result-2 filled
  ellipse spans, result-1 filled-ellipse subdivision/repeated-first-pair quirk,
  curve grouping and line input conversion. §3.3 settles Blt/BltTiled arguments
  and cleanup; §4.2 settles buffer gradient conversion. Remaining: complete
  non-AA result-1 outline subdivision and integer-line diagonal ties/clipping.
  This question is not yet fully answered.
  **Round-2 partial evidence 2026-10-24 (re-gzgraphics-gzg5b):**
  §§3.2.2–3.2.5 settle directional integer endpoint examples, ordered-chain
  polygon closure/coverage, the direct triangle fan, outline-to-cubic conversion
  and exact adaptive cubic subdivision including its N+1-segment overshoot.
  V79–V86 preserve these contracts; the earlier both-endpoints example is corrected.
  Evidence next: non-AA outline helper `0x0082DB00` and general integer-line
  helper `0x0082D040`; include diagonal ties, thin clips and clipping phase.
  Verify float widths, rounding and intermediate precision against instruction bytes.

  **ANSWERED 2026-10-26 (re-gzg29):** DIFFERS — §3.2.7 specifies the fixed
  one-unit non-AA ellipse offset, transition quads, count and numeric stores;
  integer major-axis ties and retained clipping phase have worked pixel cases.
  Narrowed remaining evidence: the exact fractional diamond ties/thin-clip
  span captures and native trig float-array capture listed in §3.2.7.
  These observations have not been run; this disposition is not full coverage.

- **GZG2 — section 4, hardware/SimGL and AA geometry:** finish the
  hardware extension-slot inventory, hardware base defaults/Shutdown, exact
  source bounds and texture/gradient subdivision, buffer AA edge coverage,
  SimGL Draw specialized/fallback routes and HWSetImage failures. Export
  pseudocode for `0x009922D0` and `0x00992F80` failed; use paged assembly.
  Relevant base helpers `0x00A64490`, `0x00A646F0`, `0x00A64A80`,
  `0x00A64DA0`, `0x00A660F0`; buffer AA `0x008308D0`.

  **ANSWERED 2026-10-26 (re-gzg29):** DIFFERS — §§4.3–4.4 settle the fallback,
  extension prefix, base stores/Shutdown and image-cache failure ownership,
  including null-source preservation of the old texture interface. Narrowed
  remaining evidence: §4.4's device-call/vertex/source-bounds captures, extension
  extent/payload captures, result-3 route and AA byte grids. No such runtime
  capture was performed; full hardware/AA compatibility is not established.

- **GZG3 — section 5, graphics IO/presentation and startup:** name and type
  remaining graphics-interface entries in the table `0x00ADB050` (slots 0–68),
  including bitmap/file IO helpers; settle each failure/output-ownership rule.
  Complete driver discovery/INI precedence, mode-selection and partial-init
  rollback, screen buffer allocation/format defaults, SetUIRenderingMode's
  exact render state/projection and RenderBufferToScreen copy/present rules.
  GetGDriverByIndex and ByID failure/ownership are settled in §5.3 (null on
  missing/out-of-range; borrowed stored pointer, no fallback or creation).
  Remaining audit includes folded driver slots
  17/20. Relevant owners are `cGZGraphicSystem` and `cGZGraphicSystemW32`,
  including unnamed helpers in region `0x00980000`.

  **ANSWERED 2026-10-26 (re-gzg29):** DIFFERS — §5.4 specifies the no-op slot
  17 and interface-relative stored-float slot 20, driver/INI preference order,
  exact ordered UI driver calls and the cached projection matrix. Narrowed
  remaining evidence: concrete W32 slot/call-site/IO ownership captures,
  per-acquisition startup failure traces and RenderBufferToScreen's copy/present
  matrix in §5.4. IO names/types and those failure rules are not claimed settled.

- **GZG5 — section 6, standalone alpha operations and loader details:** settle
  GenericBufferClip's exact rectangle/output mutation and all clipped versus
  NoClip alpha/shade/shadow/bright/stretch/half-source formulas, supported
  formats, locks, return channels and failure cleanup. Specify the legacy
  indexed surface loaders' argument widths, alpha quantization and mask byte
  meaning, malformed/short-read behaviour and resource ownership on open/lock
  failure. Next evidence: standalone helpers in `_global.c`, especially
  `0x00930BE0`, `0x00930CF0`, `0x00985D40`, `0x009888E0`, `0x00988A80`;
  confirm numeric conversions in assembly before extending the spec.
  **Partial evidence 2026-10-23 (re-gzgraphics-gzg5):** §6.3 settles ordinary
  GenericBufferClip mutation and low-byte return, the ordinary AlphaBlt wrapper's
  local rectangle copies, and legacy loader argument widths, two-division
  quantization, raw mask bytes, unchecked reads, reference leaks and invalid
  lock-failure free. **Round-2 partial evidence 2026-10-24
  (re-gzgraphics-gzg5b):** §6.3.1 now settles stretch clipping, delayed source
  mutation, half-scale single-pixel rejection, x87 truncation and collapsed-axis
  repair, with V87–V90. Remaining: standalone pixel kernels/formats/lock
  contracts. This question is not yet fully answered.

  **ANSWERED 2026-10-26 (re-gzg29):** DIFFERS — §6.2.1 adds ordinary AlphaBlt's
  lock/failure order, low-byte return, five-bit table formula and 565 green-bit
  loss, with exact worked words. The previous clipping/loader rules remain.
  Narrowed remaining evidence: per-operation slots 10–31 pixel/format/lock
  captures and allocator-dependent short-read observations listed in §6.2.1.
  The ordinary formula must not be generalized to unobserved operations.

- **GZG7 - section 3.2.2 result-1 cubic stroke quads:** specify the exact
  perpendicular corner construction, degenerate segments and intermediate
  rounding of the sixteen-interval route. [Literal cubic evaluation at i/16,
  one perpendicular half-thickness quad per interval, no joins; zero-length
  intervals submit a degenerate quad. The grouping/count contract is tested,
  but these corner details are not claimed native-exact.]

  **ANSWERED 2026-10-26 (re-gzg29):** DIFFERS — §3.2.6 specifies binary32
  coefficient/endpoint stores and x87 Horner evaluation; AA perpendicular
  corners and its length threshold; non-AA axis-dependent half-pixel caps.
  A constant AA curve submits origin-zero quads; a constant non-AA curve can
  submit nonzero rectangles. Worked cases give exact corner order.

- **GZG4 — section 7, full Windows canvas event contract:** finish the unnamed
  primary slots and subinterface thunk mappings; settle HWND ownership, window
  creation/reconfiguration failure cleanup, event pump ordering and every
  Windows keyboard/mouse/focus/resize/quit translation. Confirm names/units of
  the two words set by message `0xFA0C6390`, IID `0x1A0C615F`, subtype 13.
  Evidence next: `0x0098C690`, `0x0098CDB0`, `0x0098D080`, `0x0098D230`,
  `0x0098D9C0`, `0x0098DC40`–`0x0098DD70`; instructions are needed where
  adjusted-this and message locals are incorrectly typed in the export.

  **ANSWERED 2026-10-23 (re-gzgraphics-gzg5b):** DIFFERS — §§7.1–7.4 specify
  the native lifetime, adjusted interfaces, ordered retained listeners and full
  window-message translation. Primary slots 7/8 suspend/resume rather than
  attach/remove listeners; slot 15 emits repaint event 4 rather than pumping;
  slot 18 filters/reposts keyboard messages rather than bridging a window proc.
  Stored subtype-13 coordinates are signed client x/y pixels. Realization can
  mark itself realized on failure; the keyboard filter can consume a 1025th
  message without saving/reposting/counting it. Both quirks are contractual;
  primary slot 21 deletes the canvas rather than requesting application quit;
  vectors GZG-V62–V78 cover the newly settled behavior.

- **GZG6 — section 2.1 context destruction versus section 3.4 replacement:**
  Does final context destruction call engine Shutdown before releasing it, or
  only release the independently retained engine and target? Section 2.1 says
  destruction releases both; section 3.4 explicitly requires Shutdown only on
  replacement. The core follows that literal distinction: replacement invokes
  Shutdown, destruction releases engine then target without adding a Shutdown.
  This matters because the buffer engine's final Release does not unlock (§4.2).
  Please settle the context-destructor call sequence; no native destructor
  layout or implicit shutdown guarantee is assumed.

  **ANSWERED 2026-10-23 (re-gzgraphics-gzg5):** DIFFERS — §2.1 now specifies
  explicit engine Shutdown, temporary-vector cleanup, engine Release, then
  independent target Release. Instruction evidence: `0x0082CA40`–`0x0082CADD`.
  Direct final Release of a buffer engine still has the separate no-unlock
  quirk; context destruction explicitly shuts that engine down first.
