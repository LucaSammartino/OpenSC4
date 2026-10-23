# GZ graphics questions

Scope: [gz-graphics.md](../gz-graphics.md), Windows 1.1.641.

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
  Evidence next: helpers around `0x0082CC60`,
  `0x0082D040`–`0x0082E130`, `0x0082F970`–`0x0082FC30`; verify float widths,
  rounding and intermediate precision against instruction bytes.
- **GZG2 — section 4, hardware/SimGL and AA geometry:** finish the
  hardware extension-slot inventory, hardware base defaults/Shutdown, exact
  source bounds and texture/gradient subdivision, buffer AA edge coverage,
  SimGL Draw specialized/fallback routes and HWSetImage failures. Export
  pseudocode for `0x009922D0` and `0x00992F80` failed; use paged assembly.
  Relevant base helpers `0x00A64490`, `0x00A646F0`, `0x00A64A80`,
  `0x00A64DA0`, `0x00A660F0`; buffer AA `0x008308D0`.
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
