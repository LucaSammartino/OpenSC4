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

  **PARTIAL 2026-10-22 (re-draw-context-contracts4):** §1.3 withdraws the unsupported Begin/End request:
  renderer brackets call registered callback objects, passing the context as an argument. §§1.3–1.5
  publish lifecycle, borrowed references, attributed bodies, foreign driver targets and stack-top replay.
  Residual: attribute any native stack mutation sites/signatures (or establish that the empty vector is
  never populated); finish exact raw-matrix body `0x007D4B80` contract before claiming every owner path.

<!-- from: core-draw-context4 (2026-10-22) -->
- **DC8 - sections 6 and 8-9:** does SetModelShade update default shade and/or shade-override state,
  and how is default shade initialized/populated? Does grey bit 3 leave a previous mesh's fade multiplier
  pending? Please specify full texture-transform entries (including z and homogeneous terms), flag-combination
  details, and rounding at alpha half ties. [literal published multiplier only for SetModelShade; default shade
  remains constructor-zero until explicitly populated by renderer input; grey does not set transparency;
  matrix is identity outside the published s/t mapping; normalized alpha uses nearest with positive half up;
  transform flags combine by OR, with unset rotation/scale/translation interpreted as identity]

  **PARTIAL 2026-10-22 (re-draw-context-contracts4):** §§8.1–8.3 establish white constructor shade,
  lighting-slot-10 default population, model-light fallback and override mutation, grey's retained alpha,
  all material matrix entries, and x87 bias rounding. Whole-byte identity tests supersede per-component
  identity substitution. Residual: publish exact x87 grouping and binary32 spill points for every mixed
  compact-transform branch and the raw-matrix path `0x007D4B80`, with cancellation/scale/flag vectors.
