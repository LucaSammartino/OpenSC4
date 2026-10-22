# draw-context: open questions
<!-- Questions about [draw-context.md](../draw-context.md). Append new entries at the END; when one is answered, move it to answered/draw-context.md. Rules: README.md. -->

<!-- from: re-drawcontext (2026-10-20) -->
- **DC1 - §1.1:** what record the driver's slot 64 fills for `Init` (the DX7 driver's name for the slot is
  `GetVideoModeInfo`), and what its bytes `+0x29` and `+0x2A`, kept at `+0x78` and `+0x79`, mean. Its dword
  `+0x24` is used as the texture-stage count. [the stage count as measured; the bytes unused here]

<!-- from: re-drawcontext (2026-10-20) -->
- **DC2 - §6, §9.2:** who builds a model instance's draw-flags record and what its bits 1-5, its second byte
  (the highlight colour) and its colour at `+0x04` and float at `+0x10` stand for in game terms (fade, grey,
  hidden-part outline and constant shade are inferred from use). [the effects as measured]

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC3 - sections 1-9, owner inventory:** the engine task requires Begin/End and every owner method, but
  this spec publishes neither Begin/End signatures nor their bodies (view3d section 4 only infers a bracket).
  Please publish their lifecycle, driver calls, return values, reference ownership, shutdown/destructor behavior,
  and a complete owner-method inventory including stack operations. [Begin/End explicitly return a contract
  gap; Rust storage helpers are not asserted to be original push/pop methods; no core promotion]

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC4 - section 1.1, sections 5.2 and 6:** both combiner records have ellipses. What are their complete
  byte lengths and all bytes? Are the records 12 bytes (the space before the next published field)?
  [published bytes remain Option-valued fragments, with no invented zeros; highlight setup and material
  mode-4 default-combiner setup fail explicitly before drawing; arbitrary caller records remain opaque]

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC5 - sections 1.1, 5.2 and 7:** please publish all default stage-record values from 0x007D3C60,
  especially coordinate set, wrap/filter bytes, transform flag/matrix, texture reference ownership and
  texture-cache equality behavior. [numeric cache fields start unknown, never fabricated; first explicit
  writes reach the driver; missing values needed by second-pass replay report StageDefaults; exact initial
  cache suppression and unilateral reset remain unreconciled]

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC6 - sections 1, 3 and 7-9:** what are invalid-input and external-failure results, mutation/cleanup
  order, and retry behavior? Does every cached stage call select the stage even when no value changed,
  and is the selection repeated when transform methods call coordinate methods? [safe Rust errors for
  invalid stages, out-of-domain numeric fields and address overflow; failed commands do not advance their
  cache fields; completed prefixes remain; model drawing attempts shade/alpha cleanup and retains the first
  failure; explicit stage calls select first, cached word groups dispatch only on change]

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC7 - section 7 versus graphics-formats section 3.8:** draw-context says suppressed stages are still
  recorded, whereas graphics-formats says wrap/filter functions do nothing above stage 0 without boolean 11.
  Which cache and driver effects apply, including stages above 1 and unconditional reset loops?
  [the draw-context owner contract wins locally: record suppressed calls, suppress their driver calls;
  all allocated stage records are reset unilaterally, with driver calls gated above stage 0]

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC8 - sections 6 and 8-9:** does SetModelShade update default shade and/or shade-override state,
  and how is default shade initialized/populated? Does grey bit 3 leave a previous mesh's fade multiplier
  pending? Please specify full texture-transform entries (including z and homogeneous terms), flag-combination
  details, and rounding at alpha half ties. [literal published multiplier only for SetModelShade; default shade
  remains constructor-zero until explicitly populated by renderer input; grey does not set transparency;
  matrix is identity outside the published s/t mapping; normalized alpha uses nearest with positive half up;
  transform flags combine by OR, with unset rotation/scale/translation interpreted as identity]
