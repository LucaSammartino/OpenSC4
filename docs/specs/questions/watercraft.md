# watercraft: open questions
<!-- Questions about [watercraft.md](../watercraft.md). Append new entries at the END; when one is answered, move it to answered/watercraft.md. Rules: README.md. -->

<!-- from: re-boatsped (2026-10-21) -->
- **WC1 - §1, §5.1:** who sets the byte `+0x2CC` (count the idle budget in every animation tick); no watercraft
  function writes it except `Init`, which clears it. [as read]

<!-- from: re-boatsped (2026-10-21) -->
- **WC2 - §5.1:** what the caller of the boat's animation tick does when it returns false (a boat still for more
  than 5 s outside states 7, 16, 75–78), and the meaning of drive-parameter entries 4, 11–14, 18, 22–24, 26, 30, 31
  and 34. [as read]

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC3 - complete public owner inventory:** the implementation brief attributes 79 functions / 38556 bytes to cSC4Watercraft, but sections 1-7 and 9 name only part of them. Please publish the exact 79-method inventory, signatures, constructor/destructor and inherited-call contracts, including return values and failure paths. Which four Init flags are set (only three are named), how are failed effect creation/base Init handled, does clearing timers also clear the lifetime limit, and what version checks does Read perform? [no complete-owner coverage claim; owner initialization portion sets the three named flags and clears all named timers including limit; failed resource creation is an explicit gap; record reader supports documented version 2 only]

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC4 - sections 5.1, 5.2, 5.3 and 6.1 movement:** please expand Engine_AIBoatMotion, Movement_PositionNew and its four helpers, inherited animation/player-drive steps, and effect/model updates into executable arithmetic, ordering and failure contracts. Missing details include interpolation, steering, speed selection, water height/tilt, collision footprint construction, route-coordinate encodings, whether final docking completion is tested before or after motion, whether wake heading change means vector-delta magnitude or angle, and whether final-approach/following dt is clamped. [no original owner computation accepted as provider input; final docking completion tested also after motion, final approach dt unclamped, wake change uses vector-delta magnitude; far-approach and full animation/drive ticks remain unsupported]

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC5 - section 5.7 and automata section 6.6 crash/bump:** what are the exact footprint-overlap, inanimate hit, bump response, size-class and crash-sequence equations (sink/heel axes, ramps, RNG draws and boundary rules)? Section 6.6 gives accumulator writes but no crash animation formula. Does crash entry notify (0xB,0) as watercraft section 4 states, or (0xB,duration) as automata section 6.6 states? How are equal-size losers and non-player boats sinking after a player win handled? [literal watercraft entry notification (0xB,0), question-marked; threshold classification/state-case dispatch only, no full collision/crash-motion claim]

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC6 - section 6.2 command arithmetic:** please expand summarized speed/steering cases, input ranges/sentinels, counter-steer sign, neutral steering return, tween resets, sound selection and skier/fire positions into exact branches. In particular, how does throttle/reverse magnitude map to speed, and can braking consume leftover dt to accelerate in the opposite direction? [implement exact gear selection/coasting/button gates separately; full command translation unsupported]

<!-- from: core-watercraft4 (2026-10-22) -->
- **WC7 - sections 5.2-5.5 routes and docking:** publish the direction-to-cell/heading convention, compressed route reader format and errors, backtracking tried-bit choice, ferry exhaustion, station-vs-berth distance coordinates, initial station dock-request acquisition, and queued-with-occupied-berth behavior. When undocking cannot move/is not navigable, or docking pushes 72 on land, what does its tick return? What happens for zero-length/antiparallel heading or away vectors, including a boat exactly at the route occupant? Which station owns a newly acquired berth when current crossing still starts at the other station; when is held-berth set? For towing, which radius determines following distance, what is its translation rate, and what breaks nearest-boat distance ties? Does exiting 77 clear being_towed (not stated in section 4)? [decoded network cells only; occupied queued branch retains state; zero/antiparallel/tied cases explicitly unsupported; no guessed held-berth assignment; being_towed not cleared on exit]

<!-- from: core-watercraft-close5 (2026-10-23) -->
- **WC8 - section 6.2 literal command choices:** confirm signed gear input directly clamped to entries 6–7 as target speed; neutral brakes at entry 0 times entry 10; an opposite gear brakes to zero without reusing leftover time; left is negative and right positive, simultaneous inputs select neutral return at entry 1, and counter-steer multiplies entry 2 by entry 5. Does stopping clear the skier toggle, and can toggling while stopped arm a later start? [literal implementation: direct target; neutral braking; no leftover time; left negative/right positive; simultaneous neutral; multiplicative counter-steer; stop clears running state and toggle at rest cannot start it. Effect placement and inherited translation remain WC6.]
