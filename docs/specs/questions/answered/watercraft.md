# watercraft: answered questions



<!-- from: core-watercraft4 (2026-10-22) -->
- **WC7 - sections 5.2-5.5 routes and docking:** publish the direction-to-cell/heading convention, compressed route reader format and errors, backtracking tried-bit choice, ferry exhaustion, station-vs-berth distance coordinates, initial station dock-request acquisition, and queued-with-occupied-berth behavior. When undocking cannot move/is not navigable, or docking pushes 72 on land, what does its tick return? What happens for zero-length/antiparallel heading or away vectors, including a boat exactly at the route occupant? Which station owns a newly acquired berth when current crossing still starts at the other station; when is held-berth set? For towing, which radius determines following distance, what is its translation rate, and what breaks nearest-boat distance ties? Does exiting 77 clear being_towed (not stated in section 4)? [decoded network cells only; occupied queued branch retains state; zero/antiparallel/tied cases explicitly unsupported; no guessed held-berth assignment; being_towed not cleared on exit]

  **ANSWERED 2026-10-23 (re-watercraft-answers5):** DIFFERS — §§5.2–5.5, §6.3 and §7 give signed direction/backtracking, compressed coordinate decoding and exhaustion, station-versus-berth distances and reservation ownership, unclamped final approach/undocking/following, and degenerate-vector limits. Dock completion is before motion. Destination reservation does not set the departure record's held byte; route initialization does. The follower uses its own radius+10 endpoint gap, closes excess gap without dt scaling, and does not turn with a slack rope. The selector takes the first qualifying list entry within twice its radius, not the nearest; failed post-selection eligibility does not resume the search. Exiting 77 does not clear being-towed. Vectors WC7-D01–07/U01–03/R01–08/B01–04/T01–05 cover these rules.

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC6 - section 6.2 command arithmetic:** please expand summarized speed/steering cases, input ranges/sentinels, counter-steer sign, neutral steering return, tween resets, sound selection and skier/fire positions into exact branches. In particular, how does throttle/reverse magnitude map to speed, and can braking consume leftover dt to accelerate in the opposite direction? [implement exact gear selection/coasting/button gates separately; full command translation unsupported]

  **ANSWERED 2026-10-23 (re-watercraft-answers5):** DIFFERS — §6.2.1–§6.2.3 specify the nonlinear speed curve, ordered steering, complete command side effects, shared bookkeeping and return paths; §6.1/§6.2.3 distinguish unclamped channel/drag time from clamped movement. Opposite gear can cross zero, without a second leftover-time pass. Skier and rope use the complete boat transform with no owner-side offset; fire requests a named model action. Changed speed target clears tween elapsed and duration. Shared channel event is 0x14 through AEMS slots 6 then 5. Vectors WC8-01–13 and WC6-01–10 cover this contract.

<!-- from: core-watercraft-close5 (2026-10-23) -->
- **WC8 - section 6.2 literal command choices:** confirm signed gear input directly clamped to entries 6–7 as target speed; neutral brakes at entry 0 times entry 10; an opposite gear brakes to zero without reusing leftover time; left is negative and right positive, simultaneous inputs select neutral return at entry 1, and counter-steer multiplies entry 2 by entry 5. Does stopping clear the skier toggle, and can toggling while stopped arm a later start? [literal implementation: direct target; neutral braking; no leftover time; left negative/right positive; simultaneous neutral; multiplicative counter-steer; stop clears running state and toggle at rest cannot start it. Effect placement and inherited translation remain WC6.]

  **ANSWERED 2026-10-23 (re-watercraft-answers5):** DIFFERS — [watercraft.md §6.2.1–§6.2.2](../../watercraft.md#621-command-values-and-nonlinear-speed-response-wc8-correction) gives the tan/atan command curve, ordered speed response, and steering timer equations. Neutral leaves translator speed unchanged; coasting is a separate drive-tick operation. Opposite gear can carry speed through zero in one call. Simultaneous steering uses the ordered channel responses, with return rate entry 2 per second and counter-steer factor entry 5. Stopping stops the skier effect but does not clear its toggle input here; a still-supplied toggle can start it on a later forward-moving call. Vectors WC8-01–13 cover these distinctions. Effect placement and inherited translation remain WC6.

<!-- from: re-boatsped (2026-10-21) -->
- ~~**WC1 - §1, §5.1:** who sets the byte `+0x2CC` (count the idle budget in every animation tick); no watercraft function writes it except `Init`, which clears it. [as read]~~

  **ANSWERED 2026-10-23 (re-watercraft-wc5):** MATCHES — §5.1.2 identifies the external writer as manager PopulateVisibleArea, after successful placement. Its per-attempt mode selects group 0x4417 versus 0x4418, then is stored at +0x2CC; true accumulates second-frame time while moving and bypasses normal AI/lifetime work. RNG admission, strict mode threshold, missing-RNG behavior and idle equality boundaries are specified. Vectors WC1-I01–I05 cover activation and consumption.

<!-- from: re-boatsped (2026-10-21) -->
- ~~**WC2 - §5.1:** what the caller of the boat's animation tick does when it returns false (a boat still for more than 5 s outside states 7, 16, 75–78), and the meaning of drive-parameter entries 4, 11–14, 18, 22–24, 26, 30, 31 and 34. [as read]~~

  **ANSWERED 2026-10-23 (re-watercraft-wc5):** DIFFERS — §§2, 5.1.1 and 5.1.3 supply all listed uses. The caller invokes manager RemoveAutomaton(boat,false). Entry 18, not 24, caps normal bow pitch and supplies half its value as model lift. Entries 13/14 slew AI speed/turn per ms, 26 is a strict terrain-height contact tolerance, 30/31 are command-curve endpoints, and 34 is loaded but has no observed owner-side read. Entries 22–24 belong to crash attitude/retention. Vectors WC2-P01–P04 and corrected WC4-M07 cover the caller and newly identified uses; existing command/crash vectors cover the rest.

<!-- from: core-watercraft4 (2026-10-22) -->
- ~~**WC5 - section 5.7 and automata section 6.6 crash/bump:** what are the exact footprint-overlap, inanimate hit, bump response, size-class and crash-sequence equations (sink/heel axes, ramps, RNG draws and boundary rules)? Section 6.6 gives accumulator writes but no crash animation formula. Does crash entry notify (0xB,0) as watercraft section 4 states, or (0xB,duration) as automata section 6.6 states? How are equal-size losers and non-player boats sinking after a player win handled? [literal watercraft entry notification (0xB,0), question-marked; threshold classification/state-case dispatch only, no full collision/crash-motion claim]~~
  Round 3 progress: §§5.7.0–5.7.2 resolve boat maximum-extent gating, SAT/contact equality,
  size classes, both equal-size victims, AI sinking after player win, speed transfer/rollback,
  crash angle endpoints and distinct transform/runtime sink heights. Event (0xB,0) is confirmed;
  explicit pre-push and state-entry notifications are separate. Full shore detection,
  model-to-footprint construction and sink/heel rotation/contact remain open.
  re-watercraft-wc5 round 2: §§5.7.3–5.7.4 resolve model footprint margins,
  transform projection/offset/multiplier, missing-model failure, corner scan
  order and all 32 inanimate direction-byte cases. re-watercraft-wc5c: §5.7.2
  adds the sinking basis, independent roll/pitch signs, previous/current-heading
  roles, heading feedback and publication contract. Exact residual: grounding/heel
  rotation axes, contact transform and publication equations (0056AF90); this question stays open.

  **ANSWERED 2026-10-24 (re-watercraft-wc9):** DIFFERS — §5.7.5 completes the remaining grounding/contact/heel equations and publication contract, alongside §§5.7.0–5.7.4's earlier answers. Terrain-angle and height gates are strict; rejected contact retains the speed reduction. Contact can publish before scalar heel completion returns false. Total heel rotates the terrain normal about previous heading; incremental yaw rotates current heading about world Y. Required services and degenerate-basis behavior are explicit quirks, not unanswered alternatives. Vectors WC5-H01–H10 distinguish axes, signs, time arguments, equality and side-effect ordering.

<!-- from: core-watercraft9 round 2 (2026-10-24) -->
- ~~**WC9 - section 6.2.2 ordered steering versus WC8-08:** with L=R=100, T=0, dt=10, step 1 writes T=-0.1, then step 2's positive-T predicates both fail. Literal prose therefore leaves T=-0.1 and L=100, whereas WC8-08 requires T=-0.05 and clears L. Which predicate/ordering is intended? [implement the literal two ordered responses in section 6.2.2; encode the prose result in a question-marked test, do not claim WC8-08 conformance]~~
  **ANSWERED 2026-10-24 (re-watercraft-wc9):** DIFFERS — §6.2.2's two ordered predicates are correct. The original leaves T=-0.1 and L=R=100; the second response does not run either branch. WC8-08 now specifies that result. Instructions 0056EC87–0056ECF2 confirm the sign gates; simultaneous active inputs alone do not clear a channel.


<!-- from: core-watercraft4 (2026-10-22) -->
- ~~**WC3 - complete public owner inventory:** the implementation brief attributes 79 functions / 38556 bytes to cSC4Watercraft, but sections 1-7 and 9 name only part of them. Please publish the exact 79-method inventory, signatures, constructor/destructor and inherited-call contracts, including return values and failure paths. Which four Init flags are set (only three are named), how are failed effect creation/base Init handled, does clearing timers also clear the lifetime limit, and what version checks does Read perform? [no complete-owner coverage claim; owner initialization portion sets the three named flags and clears all named timers including limit; failed resource creation is an explicit gap; record reader supports documented version 2 only]~~
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

  **ANSWERED 2026-10-24 (re-watercraft-wc9):** DIFFERS — Scoped answer: the measured inventory is 75, and prior sections resolve the listed flags, timers, record version, constructors and failure boundaries. Round 2 adds shared tween sampling (§5.1.6), LoadModels control/results and partial failure (§3.6), and the complete shared effect-controller binding (§5.1.7). The remaining transitive model-acquisition/auxiliary/positioning audit is isolated as WC10; this archival answer does not certify complete owner closure.

<!-- from: core-watercraft4 (2026-10-22) -->
- ~~**WC4 - sections 5.1, 5.2, 5.3 and 6.1 movement:** please expand Engine_AIBoatMotion, Movement_PositionNew and its four helpers, inherited animation/player-drive steps, and effect/model updates into executable arithmetic, ordering and failure contracts. Missing details include interpolation, steering, speed selection, water height/tilt, collision footprint construction, route-coordinate encodings, whether final docking completion is tested before or after motion, whether wake heading change means vector-delta magnitude or angle, and whether final-approach/following dt is clamped. [no original owner computation accepted as provider input; final docking completion tested also after motion, final approach dt unclamped, wake change uses vector-delta magnitude; far-approach and full animation/drive ticks remain unsupported]~~
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

  **ANSWERED 2026-10-24 (re-watercraft-wc9):** DIFFERS — Scoped answer: §§5.2.1–5.2.2 now specify every AI target branch, direct translation, publication, latch and asymmetric turn-target signs. §5.1.6 specifies tween ratio rounding and NaN behavior; §5.1.7 binds executable shared controller transforms and motion parameters. Earlier sections settle the listed integration/docking/following/drive questions, including completion before motion. The only remaining model-publication helper audit is isolated as WC11; no complete-owner claim follows.
