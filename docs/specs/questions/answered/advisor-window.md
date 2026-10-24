# advisor-window: answered questions (archive)
<!-- Archive for [advisor-window.md](../../advisor-window.md). Lanes do not read this file. Open questions are in ../advisor-window.md. -->

<!-- from: re-advwin7 (2026-10-23) -->
- ~~**AW1 - §5.2:** what the list control's slots 4, 5, 6 and 7 do (`+0x5C`, child `0x100100`/`0x100101`, IID
  `0xCA1492A2`). Slot 4 takes nine arguments per advice row (two icons, body, title, a highlight flag, the advice,
  a third icon, 0, `0x14416264`). [inferred: add row, end update, set style, clear]~~
  **ANSWERED 2026-10-23 (re-advlist7):** the control is `cSC4WinAdviceList` (advice-list.md §2). Slot 4 `AddEntry`
  (open icon, closed icon, body, title, open, advice, close icon, static, background), slot 5 `RebuildHTML(keepScroll)`,
  slot 6 the background picture ID, slot 7 `ClearList`. advisor-window.md §5.2 updated.

<!-- from: re-advwin7 (2026-10-23) -->
- ~~**AW2 - §6.2:** message `0xCA09FA6C` removes the advice from the list. advisors.md §10 names `0x8A09FA6C`
  AdviceCommand; is `0xCA09FA6C` a different message (an advice dismissal)? [unknown]~~
  **ANSWERED 2026-10-23 (re-advlist7):** yes, a different message. The advice list sends `0xCA09FA6C` at once when
  the player clicks a row's close icon, after `SetFlags(8, 1)` on the advice, with the advice in the message's
  slot 33 field (advice-list.md §4). advisor-window.md §6.2 updated.

<!-- from: re-advwin7 (2026-10-23) -->
- ~~**AW3 - §6.2:** message `0x426841A1` gives a face proc a mood (data2) for advisor data1. Who posts it, and is
  data2 the same scale as advice slot 6 (0–7)? [inferred]~~
  **ANSWERED 2026-10-23 (re-advlist7):** the advice list posts it when the player toggles a row: data1 = the advice's
  advisor ID (slot 11), data2 = the advice's slot 6, the same value the window uses as the mood (advice-list.md §4).
  advisor-window.md §6.2 updated.

<!-- from: re-advwin7 (2026-10-23) -->
- ~~**AW5 - §3:** the face proc (`cSC4AdvisorUIImageWinProc`, constructor `0x00420B70`) slots 6, 7 and 8 as used
  here: visibility, a toggle value forwarded from `0x287259F8`, and the mood. [inferred]~~
  **ANSWERED 2026-10-24 (re-gzadvface7):** slot 6 `SetVisible(on)` (registers with the animation tick manager and
  loads or drops the head models), slot 7 stores `+0xF0`, the pressed offset that shifts the head and narrows its
  viewport in `DoPaint`, and slot 8 `SetMood(m)` (remapped, then the models for that mood are loaded). advisor-face.md
  §1; advisor-window.md's scope note updated.
