# audio: open questions
<!-- Engine questions for docs/specs/audio.md; append new entries below. -->

<!-- from: re-audio9 (2026-10-23) -->
- **AU40 — listener geometry and failure lifetime:** complete the camera/terrain-to-listener numerical
  contract at `0x004F14B0`, tuning fallback/array sizing at `0x004F1A80`, cheat helper `0x004F1470`,
  missing message-server handling and final message delivery/allocation path. Supplement §18 settles
  dispatch, payload shape, zoom multiply order and strict squared-distance threshold only.
- **AU41 — SEM and emanation arithmetic:** settle model slot 5/6 transformations (`0x004ED210`,
  `0x004ED2A0`, `0x004ED3F0`, `0x004ECDA0`, `0x004ECE70`, `0x004ECF90`) against instructions;
  numeric decompilation is corrupt. Recover SEM construction/defaults and registration rollback at
  `0x004EDAA0`; check caller ordering of teardown `0x004ED750` (misnamed Init in export).
- **AU42 — full scape group/instance contract:** supplement §20 inventories all 19 primary slots,
  but slots 7–17, resource format, parameter smoothing, timer widths, capacity outputs and allocation
  failures need full behavior. Relevant range `0x004F51E0`–`0x004F7B60`; service tick `0x004F67C0`.
- **AU43 — preferences construction and song/control rules:** supplement §21 settles the sound IO,
  slider arithmetic and some source/notification filtering. Recover construction/defaults, complete
  OK/cancel/revert ordering, dialog control actions/filter matrix, song discovery and exclusion-list
  persistence (`0x004F2A70`–`0x004F5120`). Existing ui-windows coverage is framework inventory only.
- **AU44 — fire-handler complete owner contract:** audio §7.4 currently gives identity and corrects
  torch ownership, rather than specifying the full fire handler. Supplement §20.3 adds some lifecycle
  and scape IO facts. Recover construction/defaults, all interface views, message dispatch and group
  registration/removal contracts at `0x004F0240`–`0x004F132F` before calling this owner complete.

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
