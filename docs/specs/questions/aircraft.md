# aircraft: open questions
<!-- Questions about [aircraft.md](../aircraft.md). Append new entries at the END; when one is answered, move it to answered/aircraft.md. Rules: README.md. -->

<!-- from: re-aircraft (2026-10-21) -->
- **AF1 - §3.1, §4:** which exemplar property holds the 18-value drive-parameter list, and what each entry means
  (speeds, turn rates, climb angles). [as read]

<!-- from: re-aircraft (2026-10-21) -->
- **AF2 - §3.4, §4:** the airplane force model (`CalculateForces`, `UpdatePhysics`) and the helicopter rotor model
  (`NextFrame` and its four steps) in full. [as read]

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF3 - §3.3 and §4, automata §6.6, AI owner closure:** please specify the complete goal switches and
  transitions, steering laws (taxi/stop/turn/follow/altitude, takeoff, landing, approach), runway direction
  clearance sampling/scoring and tie/failure rules, approach-box placement, fire search radius/drop triggers,
  helicopter reset/hover/move-XZ/building-lift controls and spotlight/chase distance gates. The chase formula
  says `pos += orient * speed * 1000 * dt` while frame dt is milliseconds: is that multiplication literal?
  Is airplane LOITER expiry `> 2000` or `>= 2000` ms; how are its timer/ammo restored after Read and what
  happens at zero ammo? [core implements the explicit chase rejection, geometry and LOITER facts; literal
  `*1000*dt` and `>2000`; missing control laws return Unsupported(AF3), without delegating them to foreign traits]

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF4 - §3.2/§3.4/§4, lifecycle and AEMS:** what are the construction defaults beyond the shared base and
  airplane drive list, the three ray-probe geometries, effect attachment/start/stop rules, Init/InitInstance/
  PhysicsInit order and allocation/failure/Shutdown cleanup? What exact sound actions map flight phases 0/1/3
  to groups 8/9/10/0xD (airplane) and helicopter rotor sounds, and what triggers phase changes?
  [core captures/restores home at first Init and returns phase-change metadata, with no repeated-phase ammo
  refill; unprovided probe, lifecycle and sound behavior remains open rather than invented]

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF5 - §3.4 and §4, U-Drive-It and spotlight:** please provide player parameter selection/reload behavior,
  command-to-control translation, takeover/release, weapon triggers/cadence/ammo/failure and projectile
  setup for jetbomb/helicoptermissle/ufolaser, and the spotlight cone/target/terrain update rules.
  [resource names and chase target geometry are available; command/weapon/spotlight updates remain Unsupported(AF5)]

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF6 - §3.3 and §4, messages and dispatch:** which aircraft-owned messages/callbacks are handled, with
  what payloads and ordering? Please give the exact flight-goal conditions that emit dispatch states 3/4/5/6/0,
  the helicopter dispatch-state setter's differences from the emergency base, and failed runway/home/fire/
  target handling. dispatch.md §4.3 lists call sites and state meanings but not the aircraft goal conditions.
  [core does not fabricate dispatch transitions or route them through a foreign AI implementation]

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF7 - §3.1, parameter-list edge cases:** what happens with a short/empty list or non-finite entries, and
  are parent chains guaranteed acyclic? Are the separately printed no-list decimal defaults exact constants,
  or recomputed from their parents? [literal finite-list implementation: missing entries act as zero;
  an existing list, including empty, stops inheritance; explicit zero uses the derived arithmetic, while no
  list uses the printed constants; no undocumented clamping or rescaling]

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF8 - §5/§8 test 3, station unit mask width:** what happens when a fire-plane station's unit count
  exceeds 32, so `2^n - 1` exceeds the available-units u32 property's width? Is there an exemplar limit,
  wider property type, clamp, or wrapping rule? [core property initialization handles 1–32 exactly,
  ignores zero/missing counts and non-building occupants, and returns Unsupported(AF8) before mutation
  for larger counts; station membership/call numbering remain foreign to the flight owners]
