# drive-tool: answered questions (archive)
<!-- Archive for [drive-tool.md](../../drive-tool.md). Lanes do not read this file. Open questions are in ../drive-tool.md. -->

<!-- from: core-player-drive4 (2026-10-22) -->
- ~~**DR7 - §7:** does message 0xA6B79602 re-enable both zoom buttons unconditionally, or refresh their CanZoomIn/CanZoomOut values as on entry (§3)? [literal re-enable sets both true on an already-created dashboard window; entry still uses the two published queries]~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — message 0xA6B79602 does not re-enable unconditionally: with a
  dashboard present it re-queries the city view's CanZoomIn and CanZoomOut and sets buttons 0x4992046A and 0xE9920494
  from those answers, exactly as on entry (0x00566890, §7); without a dashboard it does nothing. The handler answers
  "handled" either way.

<!-- from: core-player-drive4 (2026-10-22) -->
- ~~**DR5 - §5:** what is HandleKeyEffects' policy, its ordering relative to keymap result 13, Escape/repeat on release, and is keyboard command 8 handled? Does command9 toggle byte 0x229 by logical negation or another operation for values other than 0/1, and what is the event ordering relative to slot60 and that byte write? Do mouse releases share the mission gate? [normal key effects and keyboard command8 remain Unsupported; repeat suppression on key down precedes Escape; release ordering is Unsupported for repeat flags; command9 accepts only 0/1 and plans slot60 before byte write by literal table order; mouse commands share the §5 command gate]~~
  **ANSWERED 2026-10-27 (re-x-sims):** MATCHES — key order is repeat (down only), Escape (down only),
  situation-manager gate, key map (13 = unconsumed), key effects, command table (§5). Key effects (0x00566A70) are now
  specified: held byte per command at drive state +0x205, record flags 2 pulse / 1 momentary / else toggle on key
  down, flag 0x10 named effect, 0x20 controller, 0x08 consumes the key. Keyboard command 8 is not handled (right mouse
  only). Command 9 negates the byte (0 becomes 1, anything else 0) after SetDriveCommand(9, 1); release leaves it.
  Mouse buttons share the situation gate and the validity test; the left release clears the junction cell ungated
  (§5).

<!-- from: core-player-drive4 (2026-10-22) -->
- ~~**DR6 - §2, §7:** what are the hover reference/text effects on a non-null pick, panel IDs/directions, alternate-model arguments for code0xB, timer start/reset semantics, and drive-state absence behavior for mouse writes/command9? Are all automaton notifications identity-filtered, and what are unknown message/button returns? [hover identity selection and terrain-cell effects only; unresolved effects and unknown messages return Unsupported; unrelated automaton notifications do not affect current state; timer represented by ordered reset/restart markers, no invented cadence or units; missing drive state Unsupported]~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — a non-null pick only replaces the held hovered automaton (cursor text
  untouched; it is cleared on a miss). Panel messages 0xAC1F0449 (show compact, hide full), 0xCC1F0457 (show full,
  hide compact) and 0xAC1F045D (toggle on the full panel) all need a dashboard. Automaton notification 0x09D92FC3 is
  identity-checked only for code 3; code 0xB (any automaton) stops engine event 0, calls UseAlternateModel(vehicle
  view, tracked occupant, true) and restarts the millisecond timer, which ends the input at 9000 ms; code 0xD acts
  when the message's state kind is 0x10 and that automaton has no drive state. Without a drive state mouse moves write
  nothing and command 9 still sends SetDriveCommand but skips the byte. DoMessage answers handled for every message;
  DoWinProcMessage answers handled only for its four buttons (§§5-7).
