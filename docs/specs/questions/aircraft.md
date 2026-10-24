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

<!-- from: core-helicopter9 (2026-10-24) -->
- **AF9 - §4.1 rotor preset and post-Read reconstruction (AF4 follow-up):** what are the thirteen initial
  rotor preset values (exact binary32 bits), and which lifecycle step restores them and the flight phase
  after Read? The resolved zero-overlay loader requires an existing preset, but §4 does not specify one.
  [core stores unknown constructor/post-Read fields as None, accepts an explicitly supplied preset, and
  overlays known nonzero fields; zero/missing records preserve every existing field without inventing defaults]

<!-- from: glue-helicopter-shadow10 (2026-10-24) -->
- **AF10 - §4 native read-only shadow adapter:** specify the native interface IDs, exact signatures and
  ownership for reading the thirteen rotor fields and named Lua inputs, home/phase and complete goal records;
  specify a complete live-helicopter enumeration (including objects omitted by manager Write) and a safe
  deterministic scenario/group/exemplar creation and cleanup sequence. Which independent input history
  reconstructs saved home and goal transitions without seeding expected values from native outputs?
  [partial shadow compares only class identity and the four nested record version fields against a fresh
  Rust owner; scans saved manager references and uses a private uninitialized COM object as a metadata
  fixture, not a spawned city helicopter. Parameters, home, goals and live-enumeration completeness remain
  uncompared; no guessed vtable slots, memory offsets, presets or scenario group IDs]

