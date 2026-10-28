# terrain-view: open questions
<!-- Questions about [terrain-view.md](../terrain-view.md). Append new entries at the END; when one is answered, move it to answered/terrain-view.md. Rules: README.md. -->

<!-- from: automata-actions30 (2026-10-26) -->
- **TV4 - §2, §7.1; first-frame prerequisites after NM20 view Init:** which pointer is read at
  `0x007540A5`, which subsystem supplies/owns it, and which initialization, layout and lifetime
  contract must a replacement terrain satisfy before the first admitted side-cliff draw?
  `C:/Users/tlori/FreeCity4/build-runs/game-results/aa30-nm20-house-s2-b.log` records successful
  native view Init before `ready=1`: base `112BC614`, drawing interface `112BC620`, primary terrain
  `04F2C750`, live `0x24`-stride vertices at `15148040`. Both the terrain-owned and city-created
  surface-water interfaces now expose the same live depth array at `+0x18`. The first frame
  then faults at `SimCity 4.exe+0x3540A5`, reading `0x8`, with ESI=0, EDX=8, EAX=1;
  saved return addresses include `0x0075BFA3` and `0x0075C07E`. Its final verdict is corrupt,
  ref_warnings=0, 2026-01-27, **0/216** actions. The same-source DLL (`0d9c881d` SHA256 prefix)
  gives the same first-frame fault and 0/216 for s1 in `aa30-nm20-house-s1-b.log`.
  Requests are in main's `build-runs/game-queue/done/`, with the unchanged house replace list.
  §7.1 describes cliff drawing but does not identify this null read or the supplying collaborator.
  Please retain an original control's first frame and the corresponding initialization/cache writes
  before assigning ownership or a fallback. These runs cannot reach the old s2 84/216 boundary;
  NM20-A's original placement result and first pointer/cache write remain a separate runtime
  observation. [current choice: stop this item; no zero-terrain or skip-draw fallback; not accepted]
