# application: answered questions (archive)

- **AP12 - section 14.1:** is Admin Enabled=`false` case-sensitive, and does by-name loading resolve before testing?
  **ANSWERED 2026-10-23 (re-app-ap5):** DIFFERS — `false` is case-insensitive; `0` is exact. By-name loading constructs a path and calls slot 68, whose adjacent-INI check follows DLL existence testing. A bypass flag skips the check. See section 14.1 and AP12 conformance vectors.
- **AP15 - sections 2, 9, 11.6:** complete slots 51–58 and obliteration failure/view/cleanup behavior.
  **ANSWERED 2026-10-23 (re-app-ap5):** DIFFERS — slot 56 copies its own stored user-data path. Temporary-write failure does not prevent destructive close/deletion; the writer returns true on normal completion even after failed initialization/open. Read initialization is ignored, read-open failure shuts down/releases without close; absent source view defaults zoom/rotation to zero. No-city temporary writing has an unchecked city dereference, not a safe no-op. See sections 9/11.6 and AP15 vectors.
<!-- Archive for [application.md](../../application.md). Lanes do not read this file. Open questions are in ../application.md. -->

- **AP2 - section 7.1:** in `OnIdle` (`0x00449C70`), what does the 300 ms gate (+0x390 against `0x12C`) do on
  the path that sets the idle value to 60, and what does the tail that tests the paused byte +0x5D1 do? Are
  the idle message data and the set-back exactly as section 7.1 states?
  **ANSWERED 2026-10-21 (re-application4):** DIFFERS — section 7.1: +0x390 >= 300 ms (always true on a gated pass) restarts the input-idle timer and queues GameInIdleState with 60; the threshold branch sets the idle timer's elapsed to +0x3A8 - +0x3AC (set, not subtract) and is effectively unreachable; tail sleeps 250000 when +0x5D1 is set; returns false.

- **AP1 - section 8:** (narrowed 2026-10-21, re-application4: IME, r, f, w, g, d, audio/sound on, radio/music on, sim,
  RDD, CPUCount are now in section 8) which field or call does each remaining `ReadCommandLine` option
  (`0x0044EED0`) write: `ExceptionHandling`/`verbose`, `l`, `UserDir`, `CPUPriority`,
  `AutoZoomChange`, and the off paths of audio/sound and radio/music?
  DIFFERS — section 8 table: ExceptionHandling/verbose drive the exception-handler object at +0x1E8 (enable 0/1, verbose also sets option byte 6), not a prefs field; l strips :/quotes and copies <=63 chars to prefs +0x4AC; UserDir off resets the user base folder to the +0xA4 folder, else a path that passes 0x00918080 replaces it; CPUPriority low sets idle priority class 0x40; AutoZoomChange sets render property 0x25 to 0/1; audio/sound off clears prefs bytes +0x04 and +0x69 (on sets both), radio/music off clears prefs dword +0x10.
