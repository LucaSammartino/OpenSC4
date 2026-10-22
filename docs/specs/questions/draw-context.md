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
  Round 2: §8.4 now proves the raw-matrix receiver, pointer argument, four callers, every matrix entry,
  arithmetic grouping/spills and driver/failure/retry order. Residual: attribute any native stack mutation
  sites/signatures (or establish that the empty vector is never populated). No Push/Pop is inferred.
