# pedestrians: answered questions
<!-- Answered questions about [pedestrians.md](../../pedestrians.md). Keep the original entry and provenance with its answer. -->

<!-- from: re-boatsped (2026-10-21) -->
- ~~**PN1 - §2.1, §3:** what automaton slot 158 does on the first `Init`, and which code reads the one-shot
  pedestrian's play count. [as read]~~
  **ANSWERED 2026-10-24 (re-pedestrians-pn4):** DIFFERS — §2.1 specifies inherited slot 158 (`0x00544620`): one double-range draw from binary32 tuning bounds, stored as binary32 at `+0x200` before base Init when not initialized. §3 now specifies the one-shot's own animation tick (`0x00544E80`, slot 72), which directly reads/decrements the unsigned byte count at `+0x270` when the first non-null advanced model is at/past its final frame, requests `Remove(false)` at zero and returns false while still advancing later models. This corrects the former claim that only Init was overridden; it is a per-tick final-frame sample, not a completed-loop event. Successful repeat Init resets model speed and redraws the count. §8 adds PN1-I1–I10 and PN1-T1–T12 conformance vectors, including failed/repeated Init, unsigned/reversed bounds, held final frames, null entries and zero counts. No PN1 behavior remains open.

<!-- from: re-boatsped (2026-10-21) -->
- ~~**PN2 - §4.2–§4.4:** the names of the two hose effects, what the truck does with messages `0xAA0F3F75` and
  `0x8A0F3F7B` and their per-level values, and the interface `0xABF98CB3` slot 5 that the crew's strength is
  added to. [as read]~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — the hose effects are water_stream / water_sputter (level 2) and
  water_splash against fires, and foam_stream / foam_sputter and foam_splash against a toxic occupant (section 4.3);
  interface 0xABF98CB3 is the toxic occupant's, its slot 5 adds the signed hose strength to its hosed counter (4.4).
  Messages 0xAA0F3F75 and 0x8A0F3F7B are only posted (crew and riot police); the executable has no receiver for
  either, so the truck does nothing with them. The per-level values are the constants already in section 4.2.
