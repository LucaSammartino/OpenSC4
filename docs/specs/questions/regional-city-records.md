# regional-city-records: open questions
<!-- Questions about [regional-city-records.md](../regional-city-records.md). Append new entries at the END; when one is answered, move it to answered/regional-city-records.md. Rules: README.md. -->

<!-- from: re-regional-city-records (2026-10-21) -->
- **RR1 - §6.3:** the PreSave message carries the caller's flag (data4; `cSC4App::SaveCity`'s second argument, whose
  one caller `RequestCloseCity` passes the state of the Alt key after the "save before closing?" dialog). The city
  clears its "thumbnail written" bit when the flag is non-zero, and while the bit is clear and the old file has
  picture 0 it copies the old pictures instead of rendering. What is the flag meant to be (quick save, save as, a save
  from the region view), who passes a non-zero value, and is it intended that a flagged save never re-renders the
  thumbnail while an unflagged second save of a session does? [as read: copy when the flag is set or on the first
  save after a load; render otherwise]
<!-- from: re-regional-city-records (2026-10-21) -->
- **RR2 - §6.1, §6.4:** what does the second capture pass produce (pictures 4 and 6, present only in saves with
  application version 64 or 65), what does view-manager mode `0xF3` switch off (the one picture checked shows terrain
  and streets without buildings), which region-screen map mode reads these pictures, and what are the four camera
  offsets `(1/6, 1/3)`, `(1/3, -1/6)`, `(-1/6, -1/3)`, `(-1/3, 1/6)` (four tiles of one picture, or four rotations)?
  Which test decides that the game window was inactive during the capture? Where do the other colours of the border
  masks come from (picture 2 holds 32 distinct values besides white and cyan: blending, or a third ring)? [as read: one
  picture per set, the rest unknown; an engine may draw its own]
