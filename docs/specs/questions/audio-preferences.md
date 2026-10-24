# audio-preferences: open questions
<!-- Questions about [audio-preferences.md](../audio-preferences.md). Append new entries at the END; when one is answered, move it to answered/audio-preferences.md. Rules: README.md. -->

<!-- from: re-audioprefs7 (2026-10-23) -->
- **APW1 - §4.4, §4.2:** the scale of the volume words the dialog stores and sends (sound system slot 7 with
  `0x2A5B69C0`, radio slot 16 through `0xAA650860`): is 1024 (the shipped ceiling) full volume and 0 silence, and is
  the curve linear? [inferred: 0–1024 linear]

<!-- from: re-audioprefs7 (2026-10-23) -->
- **APW2 - §2.1:** the song list record word `+4` (`0xAA5FC8D5` Region, `0x2A5FC8DD` Mayor), set by `StaticInit` and
  never read by this owner: which code reads it, and is it a string ID for the list's name? [unknown]

<!-- from: re-audioprefs7 (2026-10-23) -->
- **APW3 - §6.3:** what the song buttons' slot 17 does with the buffer of the UI script key
  (`0x856DDBAC`, `0x46A006B0`, `0x14416244`), and what button slots 12 `(4, 0, 0)` and 5 `(4, 0)` set.
  [inferred: the button's look and its checkbox style]
