# occupants-props: open questions
<!-- Questions about lot-records.md (§9, §10.3, §10.4, §12), occupant-manager.md (§13.1, §13.2) and prop-occupant.md. Append new entries at the END; when one is answered, move it to answered/occupants-props.md. Rules: README.md. -->



<!-- from: re-occprops (2026-10-20) -->
- **OP6:** body of `cSC4DummyAutoOccupant` (1 fn, 65 bytes). (The immovability and zoom-and-rotation filter
  bodies are in occupant-manager.md §13.2.6, re-occfilters 2026-10-21; the dummy occupant's constructor
  `0x005B2A40` makes a point occupant on the stack of `cSC4ParticlesEffect::UpdateAttractor`, `0x005B6CA5`.)
  [untraced in this lane]


<!-- from: re-occprops (2026-10-20) -->
- **OP10:** `GetOccupantGroups` overload `0x005E9500`: it resolves group IDs through the persist-resource manager
  `0x00B43CA8` slot 0xC with IID `0xA52160F5` and then reads property `0xAA1DD396` off the resolved object
  (slot 0x24 → `0x14` → `+0x19C`/`+0x10` walk). What interface is `0xA52160F5` here (the same IID is the
  persist-resource manager per audio.md §...1145) and what does the walk return — group names or templates?
  [unresolved; both overloads fill the caller's set]

<!-- from: core-occprops4 (2026-10-21) -->
- **OP11:** lot-records.md §3.2 (u8 map counts *[quirk]*): when a lot population map (#21) or the travel-population
  map (#22) has more than 255 entries, the u8 count is truncated. Does `Write` (`0x006C0080`) then still write
  every entry (the loop runs over the map, so the record desyncs on read), or only the `count & 0xFF` first
  entries? Same for the u8 P (#20) over the vector of maps. The core writes every entry (literal reading).
- **OP12:** lot-records.md §6 base-texture `Read` resizes the record vector to n "no bound": on a short read, are
  the records beyond the failure left as constructor defaults (five `0xFF` bytes, the other fields?) and what are
  the defaults of the texture instance, cell, orientation, configuration byte and frame? The core pads with
  0 / `0xFF`×5 / 0 up to 65,536 records and leaves larger counts unpadded.


<!-- from: re-signposts (2026-10-22) -->
- **OP18 - lot-records.md §10.3:** the exact vertex layout `RebuildModel` (`0x005F5FB0`) builds: where the ten
  piece quads, the backing quad, the text quads and the caret sit relative to the position, what the lengths 8 and
  64 (screen units) are, and which table entry colours the text. [structure and sizes only]

<!-- from: re-textdecals (2026-10-22) -->
- **OP19 - lot-records.md §10.3, §10.4:** what the text texture service kept in `0x00B43CE4` (application slot
  46) is, and how its slots 4 and 5 render the text with fonts `0x2C1FD612` (signposts) and `0xCC1FD62C`
  (labels); and what `AddDecal`'s width and angle (turns) mean in overlay-manager.md's terms. [as read]
