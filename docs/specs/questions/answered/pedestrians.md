# pedestrians: answered questions
<!-- Answered questions about [pedestrians.md](../../pedestrians.md). Keep the original entry and provenance with its answer. -->

<!-- from: re-boatsped (2026-10-21) -->
- ~~**PN1 - §2.1, §3:** what automaton slot 158 does on the first `Init`, and which code reads the one-shot
  pedestrian's play count. [as read]~~
  **ANSWERED 2026-10-24 (re-pedestrians-pn4):** DIFFERS — §2.1 specifies inherited slot 158 (`0x00544620`): one double-range draw from binary32 tuning bounds, stored as binary32 at `+0x200` before base Init when not initialized. §3 now specifies the one-shot's own animation tick (`0x00544E80`, slot 72), which directly reads/decrements the unsigned byte count at `+0x270` when the first non-null advanced model is at/past its final frame, requests `Remove(false)` at zero and returns false while still advancing later models. This corrects the former claim that only Init was overridden; it is a per-tick final-frame sample, not a completed-loop event. Successful repeat Init resets model speed and redraws the count. §8 adds PN1-I1–I10 and PN1-T1–T12 conformance vectors, including failed/repeated Init, unsigned/reversed bounds, held final frames, null entries and zero counts. No PN1 behavior remains open.
