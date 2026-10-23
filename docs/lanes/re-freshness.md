| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-freshness: the Freshness grid-score ambience player (score format, cells, the player's beat clock and cell selection, cell players, fades, divisions, the dimension player), new spec freshness.md (RE spec lane) | `re-batch6` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4f059a7`, after re-hitman) | **ready** (docs and RE tables only). New docs/specs/freshness.md (§1–§9); questions FS1–FS5; 13 names (8 new); `cFresh*` claimed: specified +18,828 bytes (+203 functions) | FS1–FS5 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`),
the Ghidra export of the six `cFresh*` owners and the SDK headers. Written on 2026-10-22; the caller side (the
ambience layer's events and score list) is upstream's [audio-ambience.md](../specs/audio-ambience.md) §6 and §8.6.

### What changed

- New docs/specs/freshness.md. Byte-checked: the five CLSIDs and sizes; the only SC4 creator of a player
  (`AddFreshnessScore` `0x004E6E00`, by scanning the image for the CLSID and IID); the player and cell-player
  vtables (`0x00AD4B48`, `0x00AD48A0`) and where the port's names sit one slot off; the score separators (`\n`, `(`,
  `)`, `.`) and the `NONE` marker; the 21-entry start table (`0x00B16F30`); the 13-entry semitone table
  (`0x00B16EB0`) and the internal (divide) versus external (no divide) handling of negative steps; the pan scale
  (1/512 and 100.0); `abs` (`0x009F58E2`) in the random-volume rule.
- audio.md §2: the freshness row points at freshness.md.
- docs/specs/questions/freshness.md: FS1–FS5 (prefix new); QUESTIONS.md regenerated.

### Names and coverage

- re/symbols/freshness.tsv: 13 names (8 new: `GetCell`, `BeatTime`, `ChooseDivision`, `UpdateBeatClock`,
  `FindBestCellToStart`, the dimension player's ctor, `RebuildCellPool` and `SetScoreEnabled`; 5 confirm auto
  names). Delta (−5 +13) identical to `merge_names.py`'s.
- tools/coverage-map.tsv: `cFresh*` → freshness.md. Specified bytes 3,526,497 → 3,545,325 (63.1% → 63.5%,
  `cov_local.py`); the audio area is now 42.3% specified.

### Rebase onto `15c9ce9e` (2026-10-23)

Upstream's docs/specs/sound-system.md §6 now owns `cFreshPlayer`, `cFreshCellPlayer` and the director (§8) by
coverage row; `cFresh*` → freshness.md claims `cFreshScore`, `cFreshCell` and `cFreshDimensionPlayer`.
freshness.md states the relation: the two agree on every body both cite (`Update` = set volume then play at
`0x00947150`, slot 59 counting busy cells, the scaled-volume arithmetic, the cell states). The numbers above were
measured on `a4f059a7`.

### Questions

- New: FS1 (fields with no reader), FS2 (the start switch and suspend bytes), FS3 (the looping stop rule), FS4 (the
  callback timer's slot 1), FS5 (`SplitToken`'s not-found behaviour).

### Not covered

The dimension player in detail (unused by SC4), the external sound interface's own slots.
