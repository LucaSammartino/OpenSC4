# watercraft: open questions
<!-- Questions about [watercraft.md](../watercraft.md). Append new entries at the END; when one is answered, move it to answered/watercraft.md. Rules: README.md. -->

<!-- from: re-boatsped (2026-10-21) -->
- **WC2 - §5.1:** what the caller of the boat's animation tick does when it returns false (a boat still for more
  than 5 s outside states 7, 16, 75–78), and the meaning of drive-parameter entries 4, 11–14, 18, 22–24, 26, 30, 31
  and 34. [as read]
  Round 3 progress: watercraft §5.1.1 now identifies entries 4, 11, 12, 17–20 and 24
  through position/banking/bow arithmetic. The caller and remaining listed entries are still untraced;
  this question remains open.

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC3 - complete public owner inventory:** the implementation brief attributes 79 functions / 38556 bytes to cSC4Watercraft, but sections 1-7 and 9 name only part of them. Please publish the exact 79-method inventory, signatures, constructor/destructor and inherited-call contracts, including return values and failure paths. Which four Init flags are set (only three are named), how are failed effect creation/base Init handled, does clearing timers also clear the lifetime limit, and what version checks does Read perform? [no complete-owner coverage claim; owner initialization portion sets the three named flags and clears all named timers including limit; failed resource creation is an explicit gap; record reader supports documented version 2 only]
  Round 3 progress: §3.1 gives construction/destruction, allocation failure, all ten adjustors,
  QueryInterface, Write forwarding, sign and travel-point erase contracts. §3 already resolves
  all four Init flags, ignored base/effect results and timer reset. §10.1 measures 75 attributed
  entries, not the brief's 79. The full per-method/inherited-call contract is still incomplete.

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC4 - sections 5.1, 5.2, 5.3 and 6.1 movement:** please expand Engine_AIBoatMotion, Movement_PositionNew and its four helpers, inherited animation/player-drive steps, and effect/model updates into executable arithmetic, ordering and failure contracts. Missing details include interpolation, steering, speed selection, water height/tilt, collision footprint construction, route-coordinate encodings, whether final docking completion is tested before or after motion, whether wake heading change means vector-delta magnitude or angle, and whether final-approach/following dt is clamped. [no original owner computation accepted as provider input; final docking completion tested also after motion, final approach dt unclamped, wake change uses vector-delta magnitude; far-approach and full animation/drive ticks remain unsupported]
  Round 3 progress: §5.1.1 specifies straight/curved equations, clamping, signed yaw,
  zero/negative dt, scalar banking/bow/lift and publication ordering. Prior §5.3.1 corrects
  completion to before motion only; §§5.4.1/6.3 supply route/following rules. AI selection,
  inherited steps, full attitude composition and complete effect/model contracts remain open.

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC5 - section 5.7 and automata section 6.6 crash/bump:** what are the exact footprint-overlap, inanimate hit, bump response, size-class and crash-sequence equations (sink/heel axes, ramps, RNG draws and boundary rules)? Section 6.6 gives accumulator writes but no crash animation formula. Does crash entry notify (0xB,0) as watercraft section 4 states, or (0xB,duration) as automata section 6.6 states? How are equal-size losers and non-player boats sinking after a player win handled? [literal watercraft entry notification (0xB,0), question-marked; threshold classification/state-case dispatch only, no full collision/crash-motion claim]
  Round 3 progress: §§5.7.0–5.7.2 resolve boat maximum-extent gating, SAT/contact equality,
  size classes, both equal-size victims, AI sinking after player win, speed transfer/rollback,
  crash angle endpoints and distinct transform/runtime sink heights. Event (0xB,0) is confirmed;
  explicit pre-push and state-entry notifications are separate. Full shore detection,
  model-to-footprint construction and sink/heel rotation/contact remain open.

