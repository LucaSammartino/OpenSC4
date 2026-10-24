# advisor-window: open questions
<!-- Questions about [advisor-window.md](../advisor-window.md). Append new entries at the END; when one is answered, move it to answered/advisor-window.md. Rules: README.md. -->

<!-- from: re-advwin7 (2026-10-23) -->
- **AW1 - §5.2:** what the list control's slots 4, 5, 6 and 7 do (`+0x5C`, child `0x100100`/`0x100101`, IID
  `0xCA1492A2`). Slot 4 takes nine arguments per advice row (two icons, body, title, a highlight flag, the advice,
  a third icon, 0, `0x14416264`). [inferred: add row, end update, set style, clear]

<!-- from: re-advwin7 (2026-10-23) -->
- **AW2 - §6.2:** message `0xCA09FA6C` removes the advice from the list. advisors.md §10 names `0x8A09FA6C`
  AdviceCommand; is `0xCA09FA6C` a different message (an advice dismissal)? [unknown]

<!-- from: re-advwin7 (2026-10-23) -->
- **AW3 - §6.2:** message `0x426841A1` gives a face proc a mood (data2) for advisor data1. Who posts it, and is
  data2 the same scale as advice slot 6 (0–7)? [inferred]

<!-- from: re-advwin7 (2026-10-23) -->
- **AW4 - §5.2:** advice flag 2 (greyed when absent) and flag `0x20` (the row highlight; cleared by
  `GetMessageList` for advices without flag 2). Which states of the advice do they mean (advisors.md)? [inferred:
  active, and new/unread]

<!-- from: re-advwin7 (2026-10-23) -->
- **AW5 - §3:** the face proc (`cSC4AdvisorUIImageWinProc`, constructor `0x00420B70`) slots 6, 7 and 8 as used
  here: visibility, a toggle value forwarded from `0x287259F8`, and the mood. [inferred]
