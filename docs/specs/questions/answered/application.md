# application: answered questions (archive)
<!-- Archive for [application.md](../../application.md). Lanes do not read this file. Open questions are in ../application.md. -->

- **AP2 - section 7.1:** in `OnIdle` (`0x00449C70`), what does the 300 ms gate (+0x390 against `0x12C`) do on
  the path that sets the idle value to 60, and what does the tail that tests the paused byte +0x5D1 do? Are
  the idle message data and the set-back exactly as section 7.1 states?
  **ANSWERED 2026-10-21 (re-application4):** DIFFERS — section 7.1: +0x390 >= 300 ms (always true on a gated pass) restarts the input-idle timer and queues GameInIdleState with 60; the threshold branch sets the idle timer's elapsed to +0x3A8 - +0x3AC (set, not subtract) and is effectively unreachable; tail sleeps 250000 when +0x5D1 is set; returns false.
