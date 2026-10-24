# sound-framework: open questions
<!-- Questions about [sound-framework.md](../sound-framework.md). Append new entries at the END; when one is answered, move it to answered/sound-framework.md. Rules: README.md. -->

<!-- from: re-sndsys (2026-10-22) -->
- **SY1 - section 2.2:** what the callback timer's thread call (its slot 12, then slot 5 with 4) sets: a thread
  priority or a time resolution. [thread priority]

<!-- from: re-sndsys (2026-10-22) -->
- **SY2 - section 2.4:** which of the three listener factors pushed to renderer slots 8, 10 and 12 is distance,
  roll-off and Doppler. [in that order]

<!-- from: re-sndsys (2026-10-22) -->
- **SY3 - section 4.2:** what the instance cache's keys and lists are (which ids it maps, who fills it). [sound
  instance ids grouped by sound id]

<!-- from: re-sndsys (2026-10-22) -->
- **SY4 - section 4.5:** the meaning of the two bytes after `"XA"` in the XA file header (version, channel count or
  padding). [a version, ignored]
