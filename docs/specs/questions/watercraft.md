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
  Exact residual: tuning/debug signatures/failures, inherited initialization/teardown and
  subsidiary animation/effect/model computations. No full-owner closure claim.

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

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC5 - section 5.7 and automata section 6.6 crash/bump:** what are the exact footprint-overlap, inanimate hit, bump response, size-class and crash-sequence equations (sink/heel axes, ramps, RNG draws and boundary rules)? Section 6.6 gives accumulator writes but no crash animation formula. Does crash entry notify (0xB,0) as watercraft section 4 states, or (0xB,duration) as automata section 6.6 states? How are equal-size losers and non-player boats sinking after a player win handled? [literal watercraft entry notification (0xB,0), question-marked; threshold classification/state-case dispatch only, no full collision/crash-motion claim]
  Round 3 progress: §§5.7.0–5.7.2 resolve boat maximum-extent gating, SAT/contact equality,
  size classes, both equal-size victims, AI sinking after player win, speed transfer/rollback,
  crash angle endpoints and distinct transform/runtime sink heights. Event (0xB,0) is confirmed;
  explicit pre-push and state-entry notifications are separate. Full shore detection,
  model-to-footprint construction and sink/heel rotation/contact remain open.
  re-watercraft-wc5 round 2: §§5.7.3–5.7.4 resolve model footprint margins,
  transform projection/offset/multiplier, missing-model failure, corner scan
  order and all 32 inanimate direction-byte cases. Exact residual: sink/heel
  rotation axes, contact transform and publication equations; this question stays open.

