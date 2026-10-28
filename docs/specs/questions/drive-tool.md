# drive-tool: open questions
<!-- Questions about [drive-tool.md](../drive-tool.md). Append new entries at the END; when one is answered, move it to answered/drive-tool.md. Rules: README.md. -->

<!-- from: re-drive (2026-10-20) -->
- **DR1 - §1, §6:** the dashboard: what `ShowUI` loads and which gauges `Update` fills from which drive-state fields,
  and the four interface objects `Init` registers with the resource system. [as read]
  PARTIAL 2026-10-27 (re-x-sims): Update's gauge controls, their sources by drive-state offset, cursor selection,
  engine volumes and the 9000 ms timer are in §6; the four interface objects of Init are four small message-target
  objects (§1). Semantic names of the drive-state fields at +0x23C..+0x268, +0x144/+0x178, +0x1FF and +0x114 are not
  determined statically.
<!-- from: re-drive (2026-10-20) -->
- **DR2 - §2, §4, §5:** the meaning of the drive commands 0-12 and of the automaton slots used (13, 18, 31, 39, 40,
  50, 52, 53, 57, 59, 60, 67), and of the city situation manager's slot 32 that blocks the input. [as read]
  PARTIAL 2026-10-27 (re-x-sims): automaton slot identities verified at the call sites: 13 set state, 31 flag test,
  39/40 add/remove callback target, 52 enter state, 53 state teardown (run in StopTracking for state 0x10 when flagged
  or in group 0x4003), 57 state object by kind, 59 current state kind, 60 SetDriveCommand, 67 occupant group template
  (key effects). Command meanings 0-12 are known only as inputs (keys, sounds in §5); their effect inside the drive
  state is not read here. Situation manager slot 32 blocks all key and mouse drive input (§5).
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR3 - §1, §3, §4, §7:** what are the primitive failure, missing-service, reference ownership and reentrant-mutation contracts? In particular, are hovered automata held, are callback targets held/deduplicated, may callbacks mutate the list/current automaton, and what are the second EndInput callback value and initial native reference counts? [portable subset holds/releases only the explicitly held driven reference; ordered callback plans require caller-supplied raw values and exclude reentrant delivery; native Init/Shutdown and unknown hover ownership remain Unsupported]
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR4 - §3:** what are the three audio table values, the exact drive-settings/AEMS field selection and numeric types/boundaries for zoom/level/speed? Is property presence (including value zero) sufficient for dashboard override? [literal property presence selects the ID including zero; raw default speed bits 0x3D2C0831 retained; portable entry reports missing audio/settings/ShowUI steps, never complete native entry; nonpositive/overflowing zoom or level unsupported rather than wrapped or clamped]
<!-- from: core-player-drive4 (2026-10-23) -->
- **DR8 - §1, §4:** what are the three subscription/command IDs of table 0x00B0A110, complete native resource/service acquisition and shutdown order, and starting COM counts? In StopTracking, does "In state 0x10 ... otherwise" scope slot53 inside the state10 branch, leaving other states untouched? [native lifecycle remains Unsupported; portable cleanup uses the literal state10 scope, so non10 states have no additional state setter; only an explicitly acquired driven reference is released]
  PARTIAL 2026-10-27 (re-x-sims): the table at 0x00B0A110 holds 0xAC1F0449, 0xCC1F0457, 0xAC1F045D (§1); Init and
  Shutdown order established in §1; in StopTracking slot 53 is inside the state 0x10 branch, other states are
  untouched (§4). Starting reference counts of the created objects not determined.
