# landvalue: answered questions (archive)
<!-- Archive for [landvalue.md](../../landvalue.md). Lanes do not read this file. Open questions are in ../landvalue.md. -->

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV1:** with several growth developers, is the developers' address order the tuning's listing order? [not
  compared]~~ **ANSWERED 2026-10-14** from the exe (tuning reader `0x006BE110`; developer creation `0x006991A0` from
  the resource enumeration callback `0x00699E20`, load factory `0x006B7870`) and written into landvalue.md §7.4.
  **No.** The list is a map keyed by the developer pointer as an unsigned 32-bit value, walked from the smallest:
  ascending pointer order, fixed by the allocator. That matches neither the tuning order nor, in general, the
  creation order. A shadow reproduces it exactly by sorting the pointers `GetDeveloper(ID)` returns.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV2:** a desirability map can change between the original's pass reading it and the comparison. [rerun with the
  previous map and keep the better result]~~ **ANSWERED 2026-10-14** from the exe (growth developer slot 13
  `0x007C9FB0` returns the published grid; state 17 copies the working grid into it in one call; land value state 7
  reads and applies it in one call) and written into landvalue.md §7.4. **The bracket is sound.** The map that state
  7 sees is never half-built and changes at most once before the comparison, so there are exactly two candidates:
  the map before the developer's last state-17 copy, and the live one. The developer's slot 10 tells a shadow when to
  snapshot the first.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV3:** is `GetAreaSlope` on the lot developer free of side effects? [assumed read-only]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006C5DC0`, its vertex reader `0x006C4310`, terrain `0x007413C0`/`0x00744980`) and written into
  landvalue.md §6. **Yes, read-only**: terrain altitude and water level reads, locals only.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV4:** `GetLandValueType` is sampled at each tract's first cell. Is the type uniform over a tract, or which
  cell does the original use? [first cell]~~ **ANSWERED 2026-10-14** from the exe (`0x006BD700` calls the total grid's
  `GetCellValue`, which shifts the cell to its tract) and written into landvalue.md §7.6. **Uniform**: every cell of
  a tract gives the same value and type, so the first cell is exact.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**LV5:** LV2: a developer first seen while its slot 10 is false gets no snapshot until slot 10 turns true, so there
  is no second candidate in that window. [no snapshot]~~
  **ANSWERED 2026-10-14** from the growth developer's state machine (growth-developer §2, §3.2: slot 10 is `state < 17`,
  slot 28 is `state == 0`) and written into landvalue.md §7.4. Slot 10 is false only in states 17–20, and D changes in
  that window's first call, so a developer first seen there has already published unless it is still in state 17, which
  no slot distinguishes. The second candidate cannot be recovered. Since the state is 0 after construction, `Init` and a
  load, only a shadow that starts mid-update meets this. No original behaviour for the shadow; the implementer's choice
  (no snapshot) stands.
