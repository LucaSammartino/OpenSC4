# advisor-window: open questions
<!-- Questions about [advisor-window.md](../advisor-window.md). Append new entries at the END; when one is answered, move it to answered/advisor-window.md. Rules: README.md. -->

<!-- from: re-advwin7 (2026-10-23) -->
- **AW4 - §5.2:** advice flag 2 (greyed when absent) and flag `0x20` (the row highlight; cleared by
  `GetMessageList` for advices without flag 2). Which states of the advice do they mean (advisors.md)? [inferred:
  active, and new/unread]

<!-- from: re-advwin7 (2026-10-23) -->
- **AW5 - §3:** the face proc (`cSC4AdvisorUIImageWinProc`, constructor `0x00420B70`) slots 6, 7 and 8 as used
  here: visibility, a toggle value forwarded from `0x287259F8`, and the mood. [inferred]
