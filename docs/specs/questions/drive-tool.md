# drive-tool: open questions
<!-- Questions about [drive-tool.md](../drive-tool.md). Append new entries at the END; when one is answered, move it to answered/drive-tool.md. Rules: README.md. -->

<!-- from: re-drive (2026-10-20) -->
- **DR1 - §1, §6:** the dashboard: what `ShowUI` loads and which gauges `Update` fills from which drive-state fields,
  and the four interface objects `Init` registers with the resource system. [as read]
<!-- from: re-drive (2026-10-20) -->
- **DR2 - §2, §4, §5:** the meaning of the drive commands 0-12 and of the automaton slots used (13, 18, 31, 39, 40,
  50, 52, 53, 57, 59, 60, 67), and of the city situation manager's slot 32 that blocks the input. [as read]
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR3 - §1, §3, §4, §7:** what are the primitive failure, missing-service, reference ownership and reentrant-mutation contracts? In particular, are hovered automata held, are callback targets held/deduplicated, may callbacks mutate the list/current automaton, and what are the second EndInput callback value and initial native reference counts? [portable subset holds/releases only the explicitly held driven reference; ordered callback plans require caller-supplied raw values and exclude reentrant delivery; native Init/Shutdown and unknown hover ownership remain Unsupported]
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR4 - §3:** what are the three audio table values, the exact drive-settings/AEMS field selection and numeric types/boundaries for zoom/level/speed? Is property presence (including value zero) sufficient for dashboard override? [literal property presence selects the ID including zero; raw default speed bits 0x3D2C0831 retained; portable entry reports missing audio/settings/ShowUI steps, never complete native entry; nonpositive/overflowing zoom or level unsupported rather than wrapped or clamped]
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR5 - §5:** what is HandleKeyEffects' policy, its ordering relative to keymap result 13, Escape/repeat on release, and is keyboard command 8 handled? Does command9 toggle byte 0x229 by logical negation or another operation for values other than 0/1, and what is the event ordering relative to slot60 and that byte write? Do mouse releases share the mission gate? [normal key effects and keyboard command8 remain Unsupported; repeat suppression on key down precedes Escape; release ordering is Unsupported for repeat flags; command9 accepts only 0/1 and plans slot60 before byte write by literal table order; mouse commands share the §5 command gate]
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR6 - §2, §7:** what are the hover reference/text effects on a non-null pick, panel IDs/directions, alternate-model arguments for code0xB, timer start/reset semantics, and drive-state absence behavior for mouse writes/command9? Are all automaton notifications identity-filtered, and what are unknown message/button returns? [hover identity selection and terrain-cell effects only; unresolved effects and unknown messages return Unsupported; unrelated automaton notifications do not affect current state; timer represented by ordered reset/restart markers, no invented cadence or units; missing drive state Unsupported]
<!-- from: core-player-drive4 (2026-10-22) -->
- **DR7 - §7:** does message 0xA6B79602 re-enable both zoom buttons unconditionally, or refresh their CanZoomIn/CanZoomOut values as on entry (§3)? [literal re-enable sets both true on an already-created dashboard window; entry still uses the two published queries]
<!-- from: core-player-drive4 (2026-10-23) -->
- **DR8 - §1, §4:** what are the three subscription/command IDs of table 0x00B0A110, complete native resource/service acquisition and shutdown order, and starting COM counts? In StopTracking, does "In state 0x10 ... otherwise" scope slot53 inside the state10 branch, leaving other states untouched? [native lifecycle remains Unsupported; portable cleanup uses the literal state10 scope, so non10 states have no additional state setter; only an explicitly acquired driven reference is released]
