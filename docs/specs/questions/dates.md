# dates: open questions
<!-- Questions about [dates.md](../dates.md). Append new entries at the END; when one is answered, move it to answered/dates.md. Rules: README.md. -->

<!-- from: re-dates (2026-10-19) -->
- **DT1 - §1.6:** are the format codes of `cRZDate::DateString` (`0x00912C79`) the language manager's language
  IDs (1 US English month/day/year, 2 UK English day/month/year, dots for 4, 10 and 11, dashes for 7, 8 and 14,
  year first for 9 and 15)? Both callers in the game pass 0, so only plugins see the other codes.
