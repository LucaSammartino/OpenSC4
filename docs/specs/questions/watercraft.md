# watercraft: open questions
<!-- Questions about [watercraft.md](../watercraft.md). Append new entries at the END; when one is answered, move it to answered/watercraft.md. Rules: README.md. -->

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC3 - complete public owner inventory:** the implementation brief attributes 79 functions / 38556 bytes to cSC4Watercraft, but sections 1-7 and 9 name only part of them. Please publish the exact 79-method inventory, signatures, constructor/destructor and inherited-call contracts, including return values and failure paths. Which four Init flags are set (only three are named), how are failed effect creation/base Init handled, does clearing timers also clear the lifetime limit, and what version checks does Read perform? [no complete-owner coverage claim; owner initialization portion sets the three named flags and clears all named timers including limit; failed resource creation is an explicit gap; record reader supports documented version 2 only]
  Round 3 progress: §3.1 gives construction/destruction, allocation failure, all ten adjustors,
  QueryInterface, Write forwarding, sign and travel-point erase contracts. §3 already resolves
  all four Init flags, ignored base/effect results and timer reset. §10.1 measures 75 attributed
  entries, not the brief's 79. The full per-method/inherited-call contract is still incomplete.
  re-watercraft-wc5 round 2: §3.2 adds complete front/rear endpoint, maximum
  occupant extent and outside-city helper contracts; §5.7.3 supplies manager
  collision-search/shared footprint-builder signatures and failure behavior.
  Remaining signatures/inherited computations in §10.1 still prevent closure.
  re-watercraft-wc5b: §3.3 adds movement-gate short-circuit/state-query order,
  straight-integrator arguments/outputs and docking helper result/failure contracts.
  §5.1.4 specifies inherited animation dispatch and generic seek arithmetic.
  re-watercraft-wc5c: §3.4 completes angular/quantization signatures and
  side effects, OnLand comparison/short circuit and EndTowing reference handling.
  §9 also supplies the debug borrowed-string lifetime, signature and failure contract.
  Exact residual: tuning signature/failures, inherited initialization/teardown and
  subsidiary animation/effect/model computations. No full-owner closure claim.
  re-watercraft-wc9: §3.5 resolves the void tuning contract, missing-list defaults,
  resolved-entry-9 dependency, and inherited Init/Shutdown ordering and results.
  §5.1.5 expands slots 83/103/104/107 and the required no-tow rope-effect path.
  Residual: complete transitive LoadModels/effect-controller contracts and common
  TweenStruct sampling arithmetic; this is not complete owner closure.

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC4 - sections 5.1, 5.2, 5.3 and 6.1 movement:** please expand Engine_AIBoatMotion, Movement_PositionNew and its four helpers, inherited animation/player-drive steps, and effect/model updates into executable arithmetic, ordering and failure contracts. Missing details include interpolation, steering, speed selection, water height/tilt, collision footprint construction, route-coordinate encodings, whether final docking completion is tested before or after motion, whether wake heading change means vector-delta magnitude or angle, and whether final-approach/following dt is clamped. [no original owner computation accepted as provider input; final docking completion tested also after motion, final approach dt unclamped, wake change uses vector-delta magnitude; far-approach and full animation/drive ticks remain unsupported]
  Round 3 progress: §5.1.1 specifies straight/curved equations, clamping, signed yaw,
  zero/negative dt, scalar banking/bow/lift and publication ordering. Prior §5.3.1 corrects
  completion to before motion only; §§5.4.1/6.3 supply route/following rules. AI selection,
  inherited steps, full attitude composition and complete effect/model contracts remain open.
  re-watercraft-wc5 round 2: §5.1.1 now gives the full normal visual basis,
  correcting the bow cap/lift to p[18]; §5.1.3 gives target slew and dt use.
  Residual: complete AI target selection, inherited animation/player-drive
  computations, complete wake/effect/model updates and far approach.
  re-watercraft-wc5b: §5.1.4 resolves the generic inherited seek arithmetic,
  flag/state gates, two time arguments, unconditional normal true result and
  six-key wake request order/adapter forwarding. Residual: full AI selection,
  inherited player-drive, subsidiary slot-83/103 animation computations,
  effect-controller and slot-104/107 updates. §5.3.2 (re-watercraft-wc5c)
  supplies far-approach clamping, entry effects, call order and return contract. Normal attitude
  composition was already resolved in round 2; crash attitude remains WC5.
  re-watercraft-wc9: §5.1.5 supplies subsidiary dispatch/publication boundaries,
  §5.2.1 the three-point midpoint, strict-distance and consumption gates, and
  §5.7.5 the grounding transform. Residual: full AI target-production formulas,
  common tween sampling and effect-controller internals; WC4 remains open.

