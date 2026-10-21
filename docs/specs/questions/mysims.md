# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

<!-- from: the mysims core audit (cores26, 2026-10-20): mysims.md §11.2's open points, first filed -->
<!-- from: the cores28 lane (2026-10-20): implementing the cSC4MySimDispatch and cSC4MySimPool cores -->

<!-- from: core-mysims-contractfix4 (2026-10-21): the dispatch location contract and the pool import after MS11/MS12 -->
- **MS13 - section 4.7.1:** location details left after MS11: (a) `0x00431370` `floor(world / 16)` for a NaN or a
  quotient outside `i32` (x87 integer-indefinite `0x80000000`, or something else?) [Rust saturating cast, NaN → 0];
  (b) does `SetCellPosition(x, z)` (neighbour branch of `validateCommuteLocation`) also write the world fields
  `+0x04`/`+0x08`, or do they keep the −16.0 of the step-1 reset? [cells only]; (c) the home start rect when the home
  holds an occupant whose lot is not found: the lot at the home cell, or straight to the single cell? [single cell].
- **MS14 - section 4.12:** `Import` details left after MS12: (a) the value of the family member count `n` when
  `sims_family_nummembers` is missing or not a number (`ReadNumber` writes nothing and its result is not tested: is
  the local pre-initialised, and to what?) [0]; (b) the text of the member keys and of `family%d_face` for `k ≥ 2^31`
  in the unsigned loop (printed with `%d`, negative, or `%u`?) [`%d`]; (c) `F.ini` for a listed entry name shorter
  than 4 bytes (`Left(len − 4)` with a negative count) [empty stem]; (d) a missing `_gender` / `_zodiac` entry:
  skip the member, or an empty value? [empty value].
