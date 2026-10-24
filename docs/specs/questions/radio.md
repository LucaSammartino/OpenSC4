# radio: open questions
<!-- Questions about [radio.md](../radio.md). Append new entries at the END; when one is answered, move it to answered/radio.md. Rules: README.md. -->

<!-- from: re-radio (2026-10-22) -->
- **RA1 - section 2.1:** what event `0x4AB186E9` does to the playing sound (its slot 22 with the position from slot
  20 and the event's value: a seek, a cross-fade, or a restart). [a fade toward the new station]

<!-- from: re-radio (2026-10-22) -->
- **RA2 - section 2.4 step 3:** which radio field's bit 0 sets the stations' shuffle strategy (1 = sort by file
  time). [an option saved in the settings record]

<!-- from: re-radio (2026-10-22) -->
- **RA3 - section 2.6:** who sets the suspend byte `+0x54` and the "keep playing" flag `+0x6D` the idle tick and
  station selection test. [the SC4 audio event handler]

<!-- from: re-radio (2026-10-22) -->
- **RA4 - section 3.2:** confirm in a running game that a station with a `Jingle\` folder never plays a jingle
  (the option-2 test). [never]
