# Growth / development pipeline (RCI building growth)

Status: work in progress (being filled incrementally). Legend: **[confirmed]** seen in code (Mac named pseudocode and/or Windows disassembly) · **[inferred]** consistent with evidence, not proven.
Windows addresses refer to `SimCity 4.exe` 1.1.641; Mac addresses (`mac:`) refer to the named Mac Rev A Intel build. Symbol map: [`re/symbols/development.tsv`](../../re/symbols/development.tsv).

## Outcome enum (cISC4TractDeveloper::Outcome) [confirmed]

From the debug string switch at `0x004CDF80` (used by the "debug growth" dump):

| # | name | # | name |
|---|---|---|---|
| 0 | unknown failure | 14 (0x0E) | stage too big |
| 1 | nothing to reoccupy | 15 | resident has different purpose |
| 2 | monopoly piece | 16 | resident has different wealth |
| 3 | no lot | 17 | not bigger than existing stage |
| 4 | lot not ready | 18 | historical |
| 5 | no lot configuration | 19 | just built recently |
| 6 | wrong zone | 20 (0x14) | not enough construction resources |
| 7 | wrong purpose | 21 | poor roads |
| 8 | wrong wealth | 22 | wrong land value |
| 9 | wrong style | 23 | no fire coverage |
| 10 | too steep | 24 | no park |
| 11 | no building big enough | 25 | no water |
| 12 | stage too small | 26 | no power |
| 13 | all buildings too big | 27 (0x1B) | **success** |

(Sections below are being written.)
