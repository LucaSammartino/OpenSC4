# audio: open questions
<!-- Engine questions for docs/specs/audio.md; append new entries below. -->

<!-- from: re-audio9 (2026-10-23) -->
- **AU42 — full scape group/instance contract:** supplement §20 inventories all 19 primary slots,
  but slots 7–17, resource format, parameter smoothing, timer widths, capacity outputs and allocation
  failures need full behavior. Relevant range `0x004F51E0`–`0x004F7B60`; service tick `0x004F67C0`.
- **AU43 — preferences construction and song/control rules:** supplement §21 settles the sound IO,
  slider arithmetic and some source/notification filtering. Recover construction/defaults, complete
  OK/cancel/revert ordering, dialog control actions/filter matrix, song discovery and exclusion-list
  persistence (`0x004F2A70`–`0x004F5120`). Existing ui-windows coverage is framework inventory only.

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU45 — completed dispatch versus vector 12:** section 7.2's completed dispatch table routes
  `0x49E64801` to HandleAutomataMessages as an accepted no-op, while section 16 vector 12 says only
  four other types may reach that helper. Should vector 12 include this fifth no-op type?
  [Implement the completed section 7.2 table literally; keep the legacy four-operation fixture list.]

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU46 — sound-manager missing owner bodies:** sections 8.1–8.3 inventory a focus flag,
  service pointers, Update callbacks and event routes, but do not define the payload gates,
  state transitions, return values or foreign slot calls for reset (`0x4A4D35E0`), enumeration
  (`0x4A4FE6F6`), volume (`0x4A451915`), focus (`0x6A95CBA2` and focus message), and the
  `0xCAA44CA7` stop-all helper. What are those algorithms, the per-sound update callback
  (`0x004F9690`), completion/registration lifetime, constructor flag defaults, and Init/Shutdown
  failure ordering? Section 10.6 supplies lookup/start facts, not these contracts.
  [Retain existing instance-table fixture; do not guess these missing owner bodies or claim
  full SoundManager closure. Implement settled routing/state/IO separately while awaiting answers.]

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU49 — start/position/stop instance completion:** §8.3 derives pause radius from the start
  parameter's low byte, but §10.6 step 4 assigns it from buffer property 0x38. What is the exact
  order and final value? Supply the tSound+14 constructor word passed to play(+14), buffer/QI and
  allocation failure paths, completion registration/unregistration calls, stop +20/+24 handle
  operations, and SetPosition's state fields/property calls/return semantics. The existing local
  fixture has no foreign position effect; the new resolve/play stages cannot close these bodies
  without those contracts. [Keep local fixture separate; implement known get-sound/QI/property/
  play operations as explicit stages, never assume missing word+14 or successful native dispatch.]

<!-- from: core-audio-close13 (2026-10-24) -->
- **AU50 — §8.3.1 remaining SoundManager algorithms:** the AU46/AU49 answer says
  UpdateAll applies distance/simulation-pause policies, linked UpdatePosition performs a
  mode-specific volume/pan update, and focus gain calls Resume, but gives none of those
  bodies. §10.8 names voice-count gates without specifying comparisons, rejection/linking
  or existing-instance selection. Please specify these algorithms, linked averaging and
  promotion state/reference transfer, Resume's foreign slots/transition rules, the six
  constructor position-sentinel bits, and start/position payload reference/return rules.
  [Implement only specified stages; record explicit Unsupported for voice policy, linked
  position, distance/simulation pause and Resume. Require sentinel as an explicit input;
  do not claim complete SoundManager execution or invent foreign Resume slots.]
