# aircraft: open questions
<!-- Questions about [aircraft.md](../aircraft.md). Append new entries at the END; when one is answered, move it to answered/aircraft.md. Rules: README.md. -->

<!-- from: re-aircraft (2026-10-21) -->
- **AF2 - §3.4, §4:** the airplane force model (`CalculateForces`, `UpdatePhysics`) and the helicopter rotor model
  (`NextFrame` and its four steps) in full. [as read]
  Partial progress (re-aircraft-af5b, 2026-10-23): §3.4.1 specifies physics seconds,
  tiny-speed/max-speed limiting, candidate integration and correction ordering;
  §3.4.2 specifies the scalar force terms, coefficient angle/curve, rolling
  resistance and reverse-throttle gate. Still open: complete attitude/lift frame,
  angular update, terrain/runway/collision corrections, and the helicopter model.

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF3 - §3.3 and §4, automata §6.6, AI owner closure:** please specify the complete goal switches and
  transitions, steering laws (taxi/stop/turn/follow/altitude, takeoff, landing, approach), runway direction
  clearance sampling/scoring and tie/failure rules, approach-box placement, fire search radius/drop triggers,
  helicopter reset/hover/move-XZ/building-lift controls and spotlight/chase distance gates. The chase formula
  says `pos += orient * speed * 1000 * dt` while frame dt is milliseconds: is that multiplication literal?
  Is airplane LOITER expiry `> 2000` or `>= 2000` ms; how are its timer/ammo restored after Read and what
  happens at zero ammo? [core implements the explicit chase rejection, geometry and LOITER facts; literal
  `*1000*dt` and `>2000`; missing control laws return Unsupported(AF3), without delegating them to foreign traits]
  Partial resolution (re-aircraft-af10, 2026-10-24): aircraft §§3.3.1/4.2.1/4.2.2
  and vectors 17–20 settle reset/hover, chase lead and 192/32/opacity gates,
  timer/ammo persistence and zero ammo. **DIFFERS:** chase dt is already seconds;
  airplane LOITER expires at remaining countdown **≤0**, not strict elapsed >2000.
  Still unknown: airplane steering, takeoff/landing/approach, clearance scoring,
  approach-box and fire/drop geometry; helicopter MoveXZ laws and
  complete non-chase goal/claim/RESCUE transitions and ordering. No full AF3 closure.
  Final-round addition: aircraft §4.2.4 and vector 24 settle building-lift
  sampling, height combination, home floor and descending demand, including
  the below-clearance-plane quirk. The auxiliary height provider's physical
  interpretation remains unlabelled; its call contract is explicit.

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF4 - §3.2/§3.4/§4, lifecycle and AEMS:** what are the construction defaults beyond the shared base and
  airplane drive list, the three ray-probe geometries, effect attachment/start/stop rules, Init/InitInstance/
  PhysicsInit order and allocation/failure/Shutdown cleanup? What exact sound actions map flight phases 0/1/3
  to groups 8/9/10/0xD (airplane) and helicopter rotor sounds, and what triggers phase changes?
  [core captures/restores home at first Init and returns phase-change metadata, with no repeated-phase ammo
  refill; unprovided probe, lifecycle and sound behavior remains open rather than invented]
  Partial resolution (re-aircraft-af10, 2026-10-24): aircraft §§3.5/4.2.3 and
  vectors 21–22 settle both phase setters' ordered sound requests, ammo refill,
  helicopter engine flag and repeated-phase behavior. Helicopter repeats its
  side effects; airplane repeats are no-ops. AF9 is already answered in §4.1.1.
  Still unknown: remaining constructor fields, ray geometry, attachment ownership,
  allocation/failure/cleanup and Init/PhysicsInit order beyond AF9, and all
  physics/collision-driven phase/audio triggers. Airplane Init/Shutdown and
  helicopter Init/Shutdown pseudocode fail in the available class exports;
  settling them requires instruction review. No full AF4 closure.
  Final-round addition: aircraft §4.2.5 and vector 25 settle first runtime
  allocation ownership, non-transactional allocation failure, post-base
  Shutdown cleanup and false-result propagation. Remove those from the
  residual above. Remaining lifecycle evidence needed: constructor defaults,
  ray probes, attachment creation, pre-base Shutdown scale handling, full
  Init/PhysicsInit order and physics/collision-driven phase triggers.

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF5 - §3.4 and §4, U-Drive-It and spotlight:** please provide player parameter selection/reload behavior,
  command-to-control translation, takeover/release, weapon triggers/cadence/ammo/failure and projectile
  setup for jetbomb/helicoptermissle/ufolaser, and the spotlight cone/target/terrain update rules.
  [resource names and chase target geometry are available; command/weapon/spotlight updates remain Unsupported(AF5)]
  Partial resolution (re-aircraft-af10, 2026-10-24): aircraft §4.3 and vector 23
  settle owner weapon request order and success/failure accounting; `ufolaser`
  follows missile failure rather than a UFO type test. Still unknown: parameter
  selection/reload and takeover/release, full command/control mapping, base
  readiness/cadence, effect attachment/projectile setup and spotlight cone,
  target smoothing and terrain transforms. No full AF5 closure.
  Final-round addition: aircraft §§4.3.1–4.3.2 and vectors 26–27 settle the
  helicopter takeover helper's record selection and five-field refresh, and
  spotlight pair activation/status/start/stop ordering. Remaining AF5 scope:
  airplane parameter refresh, takeover/release scheduling, complete command
  translation and base readiness/cadence, effect/projectile construction,
  spotlight aiming/transforms/terrain projection. Activation alone must not
  be reported as a complete spotlight update.

<!-- from: core-aircraft5 (2026-10-23) -->
- **AF6 - §3.3 and §4, messages and dispatch:** which aircraft-owned messages/callbacks are handled, with
  what payloads and ordering? Please give the exact flight-goal conditions that emit dispatch states 3/4/5/6/0,
  the helicopter dispatch-state setter's differences from the emergency base, and failed runway/home/fire/
  target handling. dispatch.md §4.3 lists call sites and state meanings but not the aircraft goal conditions.
  [core does not fabricate dispatch transitions or route them through a foreign AI implementation]

<!-- from: core-airplane9 (2026-10-24) -->
- **AF10 - §3.4.1/§3.4.2 non-finite physics inputs:** the velocity-limiter table explicitly covers finite inputs
  only. What are the comparison/normalization and time-cap results for NaN/infinite velocity, maximum speed,
  milliseconds and AI multipliers? Should unordered x87 comparisons choose a branch different from ordinary
  floating comparisons? [finite vectors are implemented; isolated helpers use literal comparisons without
  sanitization, while the complete flight tick remains Unsupported(AF2); no non-finite native equivalence claimed]

<!-- from: glue-helicopter-shadow10 (2026-10-24) -->
- **AF11 - §4 native read-only shadow adapter:** specify the native interface IDs, exact signatures and
  ownership for reading the thirteen rotor fields and named Lua inputs, home/phase and complete goal records;
  specify a complete live-helicopter enumeration (including objects omitted by manager Write) and a safe
  deterministic scenario/group/exemplar creation and cleanup sequence. Which independent input history
  reconstructs saved home and goal transitions without seeding expected values from native outputs?
  [partial shadow compares only class identity and the four nested record version fields against a fresh
  Rust owner; scans saved manager references and uses a private uninitialized COM object as a metadata
  fixture, not a spawned city helicopter. Parameters, home, goals and live-enumeration completeness remain
  uncompared; no guessed vtable slots, memory offsets, presets or scenario group IDs]

