# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

<!-- from: the mysims core audit (cores26, 2026-10-20): mysims.md §11.2's open points, first filed -->
<!-- from: the cores28 lane (2026-10-20): implementing the cSC4MySimDispatch and cSC4MySimPool cores -->

<!-- from: core-mysims-pool4 (2026-10-21) -->
- **MS11 - section 4.7.1:** dispatch location details the job-search core needs: (a) when is a location "valid"
  for `SetCurrentCommuteDestination` step 1 beyond "the default (occupant flag set, cells -1) is invalid"?
  [occupant flag set: valid iff an occupant is held; flag clear: valid iff both cells >= 0]; (b) the default bytes of
  the neighbour-city fields `+0x14`/`+0x18` [not written; modelled as absent]; (c) what cell `SetByOccupant`
  stores (the lot cell reported with `0xEBA06BF4`) [the search's end cell]; (d) is dispatch `+0x40` the trip state of
  section 4.6 [kept as its own field]; (e) the `notify` argument `ChangeJob(b)` passes [false: no `0xEBA06BF4`].
<!-- from: core-mysims-pool4 (2026-10-21) -->
- **MS12 - section 4.12:** pool details left open: (a) the template fields when an exemplar property is absent
  (`0xEA296F8D`, `0xEA296F8F`, `0xEA296F91`, `0xEA296F90`) [0 / empty / false / 0]; (b) the ini reader's section and
  key matching (case) and its integer parse for `sims_family_nummembers` / `_guid` [exact bytes; atoi-like decimal];
  (c) the exact bytes of the picture folder ("install path, `Web Pages` and `FamilyGFX` with that value") and of the
  `family<k>_face.jpg` source [`<install>\Web Pages\FamilyGFX\<value>\family<k>_face.jpg`]; (d) `F.ini` directory
  [the `Export` directory of `F.pub`]; (e) `Import` record strings longer than 255 bytes [cut to 255];
  (f) `<path>\UserData` when the registry path already ends in `\` [plain concatenation, double separator kept];
  (g) the "male" text or a zodiac name failing to load [empty string].
