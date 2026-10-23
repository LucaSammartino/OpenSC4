# occupant-manager: answered questions (archive)
<!-- Archive for [occupant-manager.md](../../occupant-manager.md). Lanes do not read this file. Open questions are in ../occupant-manager.md. -->

## From the occupant manager and automata cores (filed by re-answers16 from the coordinator's task, 2026-10-16)

The occupant manager questions OM4–OM8 continue occupant-manager.md §16. AT2–AT5 are automata.md §13 Q2–Q5,
filed here under their own prefix. The implementers' choices are in brackets.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**OM4:** is the 0.1 m shrink constant (occupant-manager §5.1 step 5 and the standard-cell boxes of slots
  32/34/40) `float` or `double`, and at which address? [float]~~ **ANSWERED 2026-10-16 (re-answers16):** MATCHES.
  It is `float` `0x3DCCCCCD` at `0x00A8F768`, read as a 4-byte operand in Insert, Move, the FindOccupant helper and
  slots 32/34/40/47. The 1.0 width limit is `float` at `0x00A81228`. Every shrunk bound is stored as a `float`.
  occupant-manager §5.1, §7.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**OM5:** slot 34's clamp to the standard counts: 0..=count or 0..=count−1? [0..=count]~~ **ANSWERED 2026-10-16
  (re-answers16):** **DIFFERS**. A non-null min or max is clamped to 0…count − 1 (`0x0049E827`–`0x0049E8B7`). A null
  max becomes (count X, count Z), one standard cell past the edge, as in slot 40. occupant-manager §7, test 21.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**OM6:** out-of-grid and crash paths: Insert before Init; manager boxes with bytes past 191 or negative cells;
  slot 23 with a null filter; slot 28 outside the grid; FindOccupant on an occupant without a property holder. [no
  nodes / flag 1 set and Insert posted / true and no-op / none / no guess]~~ **ANSWERED 2026-10-16 (re-answers16):**
  occupant-manager §5.7 and §6.3. Where the original is defined, the brackets MATCH:
  - a **wrapped** box (low corner byte > high, e.g. a cell below 0 stored as 255) gets no nodes, while flag 1 is set
    and Insert is posted (loops `0x0049F0C8`/`0x0049F0EA`);
  - FindOccupant without a property holder guesses from two uninitialised locals; Insert and Move never place such
    an occupant, so skipping the guess gives the same results.

  The remaining cases crash or touch wild memory in the original, so the brackets are accepted as choices:
  - Insert before `Init` reads through the null grid at `0x0049F0F7`, after Create and the stored box. Insert after
    `Shutdown` works, because the rows survive.
  - Bytes 192–255 inside a running loop read past the grid in Insert, Remove and Move.
  - Slot 23 writes the type to address `0xC` (`0x0049DB87`).
  - Slot 28 (and slot 32) index the grid unclamped.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**OM7:** does slot 44 `IterateOccupants` skip the duplicate check for single-cell nodes like slots 36/38? [checks
  every node]~~ **ANSWERED 2026-10-16 (re-answers16):** **DIFFERS** in mechanism, with identical results:
  - it skips the check for bit-0 nodes (`0x004A1689`–`0x004A1695`);
  - a single-cell walk has no check at all;
  - the spec's §9.2 was also wrong about callbacks: slot 44 collects an AddRef'd batch per cell under an inner lock
    and calls back after dropping it, all inside the outer lock. So a callback may remove the occupant it was given.
    occupant-manager §9.2, test 23.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**OM8:** do `Write`/`Read` and slot 25 take the manager's lock? [yes]~~ **ANSWERED 2026-10-16 (re-answers16):**
  **DIFFERS**: none of them does (`0x0049ECF0`, `0x004A1A20`, `0x0049E490`). The extra recursive lock is not
  observable and may stay. occupant-manager §13, §14.

<!-- from: re-occfilters (2026-10-20) -->
- ~~**OM10 - §13.2.3:** which cheat word reaches the 3D view's code at `0x007F365D` that installs a realistic filter
  with all three flags clear? [as read]~~
  **ANSWERED 2026-10-21 (re-view3dwin):** DIFFERS — `Zoneria` (`0x6A7A276E`): its new realistic filter starts with all
  three flags clear, then, when the current view filter is realistic, flips `+0xC` and copies `+0xD`, leaving `+0xE`
  clear; `TastyZots` (`0x007F391F`) flips `+0xD` and copies the other two (view3d-window.md §3.1, occupant-manager.md
  §13.2.3).
