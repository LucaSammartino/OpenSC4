# draw-context: answered questions

<!-- from: core-draw-context4 (2026-10-22) -->
- ~~**DC8 - sections 6 and 8-9:** does SetModelShade update default shade and/or shade-override state,
  and how is default shade initialized/populated? Does grey bit 3 leave a previous mesh's fade multiplier
  pending? Please specify full texture-transform entries (including z and homogeneous terms), flag-combination
  details, and rounding at alpha half ties. [literal published multiplier only for SetModelShade; default shade
  remains constructor-zero until explicitly populated by renderer input; grey does not set transparency;
  matrix is identity outside the published s/t mapping; normalized alpha uses nearest with positive half up;
  transform flags combine by OR, with unset rotation/scale/translation interpreted as identity]~~

  **ANSWERED 2026-10-22 (re-draw-context-contracts4):** DIFFERS — §§8.1–8.4 specify white initial
  default shade, lighting population/fallback, override and retained-grey-alpha effects, every material
  matrix entry, x87 bias rounding, whole-byte transform branches, all nine rotation groupings and
  asymmetric translation spills (x/y after scale, z before scale). Raw-matrix load has distinct grouping,
  explicit zero products, no compact-scale application, a final transpose and no MatrixMode call.
  Driver failures returning normally preserve preceding mutations and clear valid; equal retry reloads.
  §13.2 adds independent cancellation, spill, scale, flag, state and failure/retry vectors DC-V15–V26.

  The round-1 partial finding established shade/material/flag rules; round 2 closes its exact mixed-
  compact and raw-matrix arithmetic residual. Numeric rules use ambient x87 control, with vectors
  explicitly selecting nearest/even and 64-bit significand precision; no universal half-up is asserted.

<!-- from: core-draw-context4 (2026-10-22) -->
- ~~**DC4 - section 1.1, sections 5.2 and 6:** both combiner records have ellipses. What are their complete
  byte lengths and all bytes? Are the records 12 bytes (the space before the next published field)?
  [published bytes remain Option-valued fragments, with no invented zeros; highlight setup and material
  mode-4 default-combiner setup fail explicitly before drawing; arbitrary caller records remain opaque]~~

  **ANSWERED 2026-10-22 (re-draw-context-contracts4):** DIFFERS — §1.2 gives every byte of the
  12-byte owner constructor/Init and stage-default records. Trailing zeros are proved word stores;
  there are no unknown owner combiner bytes requiring a setup failure.

<!-- from: core-draw-context4 (2026-10-22) -->
- ~~**DC5 - sections 1.1, 5.2 and 7:** please publish all default stage-record values from 0x007D3C60,
  especially coordinate set, wrap/filter bytes, transform flag/matrix, texture reference ownership and
  texture-cache equality behavior. [numeric cache fields start unknown, never fabricated; first explicit
  writes reach the driver; missing values needed by second-pass replay report StageDefaults; exact initial
  cache suppression and unilateral reset remain unreconciled]~~

  **ANSWERED 2026-10-22 (re-draw-context-contracts4):** DIFFERS — §§1.4, 7.2–7.3 establish borrowed
  texture value 0xFFFFFFFF, coordinates 0, wrap/filter −1, false transform flag, identity saved matrix,
  explicit combiner bytes and unspecified three-byte padding. Binding has no equality suppression;
  initial coordinate(0) is silent. Reset bypasses ordinary stage gates before restoring the records.

<!-- from: core-draw-context4 (2026-10-22) -->
- ~~**DC6 - sections 1, 3 and 7-9:** what are invalid-input and external-failure results, mutation/cleanup
  order, and retry behavior? Does every cached stage call select the stage even when no value changed,
  and is the selection repeated when transform methods call coordinate methods? [safe Rust errors for
  invalid stages, out-of-domain numeric fields and address overflow; failed commands do not advance their
  cache fields; completed prefixes remain; model drawing attempts shade/alpha cleanup and retains the first
  failure; explicit stage calls select first, cached word groups dispatch only on change]~~

  **ANSWERED 2026-10-22 (re-draw-context-contracts4):** DIFFERS — §§7.2–7.4 specify actual ordering.
  Equal coordinate/wrap/filter requests send no selection; transforms can select twice. Native stage
  addressing is unchecked. Listed driver return registers are ignored and caches advance on normal
  return, including failure-valued returns. Equal retry can be suppressed. Model cleanup runs at the
  normal loop end, with no first-failure accumulator or exception cleanup guarantee.

<!-- from: core-draw-context4 (2026-10-22) -->
- ~~**DC7 - section 7 versus graphics-formats section 3.8:** draw-context says suppressed stages are still
  recorded, whereas graphics-formats says wrap/filter functions do nothing above stage 0 without boolean 11.
  Which cache and driver effects apply, including stages above 1 and unconditional reset loops?
  [the draw-context owner contract wins locally: record suppressed calls, suppress their driver calls;
  all allocated stage records are reset unilaterally, with driver calls gated above stage 0]~~

  **ANSWERED 2026-10-22 (re-draw-context-contracts4):** DIFFERS — §§7.2–7.3: ordinary wrap/filter
  calls cache requested values on all allocated stages, while their driver calls require stage 0 or
  boolean 11. Unilateral reset selects/clears/coordinates every stage without that gate; clear-transform
  also bypasses the gate for its explicit select/clear sequence. Mode setup above stage 1 and combiner
  submission have separate ungated paths. Property changes do not invalidate requested caches.
