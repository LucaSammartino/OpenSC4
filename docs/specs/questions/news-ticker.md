# news-ticker: open questions
<!-- Questions about [news-ticker.md](../news-ticker.md). Append new entries at the END; when one is answered, move it to answered/news-ticker.md. Rules: README.md. -->

<!-- from: re-ticker7 (2026-10-23) -->
- **NTK1 - §4, §5.1:** advice slot 7, which orders the list and gates the flipper (above 59). Is it the advice's
  priority (advisors.md)? [inferred]

<!-- from: re-ticker7 (2026-10-23) -->
- **NTK2 - §3:** advice flag `0x2000` keeps an added advice out of the ticker. Which advices carry it? [unknown]

<!-- from: re-ticker7 (2026-10-23) -->
- **NTK3 - §5.3:** language 15 (`RZ::LM` slot 7) switches the flipper to the smaller `SIZE=2` face. Which language
  is 15? [inferred: an East Asian language]

<!-- from: re-ticker7 (2026-10-23) -->
- **NTK4 - §2:** the timer callback hands the `0x0A344006` message and the ticker to message server 2's slot 7.
  Is that a synchronous send to that target? [inferred]
