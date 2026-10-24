| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-weather-we1: weather rounding contract | Codex / `re-weather-we1` (RE, detached HEAD) | in progress | Resolve WE1 wind and moisture precision from original instructions |

## Handoff
- Goal: close WE1 and later moisture precision divergence with behavior rules and bit-pattern conformance vectors; no engine edits.
- Read engine lane closure context: runs 374–377 first differ at step 3 wind x, saved byte 65833, 118/117; all weather comparisons 1/73.
- Source question: `C:/Users/tlori/FreeCity4/.claude/worktrees/glue-weather-propdev6/docs/specs/questions/weather.md`.
- WE1 copied with all four run continuations into this worktree's question file.
- Verified daily arithmetic: normal 24-bit x87 PC (foundations §18); multiply by binary64 1/30, round every arithmetic operation; normalize with rounded reciprocal then component products (`0x0072981A–34`).
- Additional moisture issue: `GetAmbientWind` re-normalizes direction with `ScaleVector2D` (`0x00725A28`, `0x005D76D0`), with threshold 0.01f. Existing §4/§6.5 omitted this.
- Moisture slope rounds z product + x product, adds y normal times zero, multiplies slopeK, subtracts from integer b, then uses 12582912.0f bias for nearest-even integer conversion (`0x00727898–E8`).
- Next: write §5.1/§6.5 exact equations and conformance tables, then archive WE1 and run doccheck. Scratch exact rational vector calculator: `build-runs/we1-arithmetic.py`.
- Main checkout sources read only; detached HEAD reserved by supervisor. No ABI changes or game requests.
