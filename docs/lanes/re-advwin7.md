| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advwin7: the advisor window `cSC4AdvisorUIAdvisorWinProc`, new spec advisor-window.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-regionview7) | **ready** (docs only). advisor-window.md §1–§8: identity, QI/Release and the destructor, the layout and the advisor/face/button tables, `Init`, faces and moods, the three layouts, the advice list (maintenance, subscription, the HTML body builder with its colour/icon/mood table), `DoWinProcMessage` and `DoMessage`, quirks, six conformance cases; questions AW1–AW5; owner claimed: `cSC4AdvisorUIAdvisorWinProc`: specified +10,177 bytes (+26 functions) | AW1–AW5 |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 26 claimed functions were disassembled, plus the constructor, show and teardown that advisors.md §12.1.1 already
describes (owned by the advisor system), and the face proc's constructor. The HTML strings, the advisor ID table,
the mood jump tables and the vtables were read from the data section. Written on 2026-10-23. No upstream lane
covers this owner (advisors.md §12.1.1 describes only the system's slots 23–28 and the direct-call ABI; this spec
gives the bodies).

### What changed

- New docs/specs/advisor-window.md.
- Read on the bytes:
  - the layout (advisor, ID and index, script window, the seven face generators and procs, the flags, the layout
    code, the list control, the advice list and map);
  - the advisor ID table `0x19, 0x15–0x18, 0x1A, 0x1B` with its face windows and buttons;
  - `Init` (script window `0x96A006B0/0xCBC905CD`, the alignment of the new children, focus flags, the three layout
    panels);
  - the mood rule (minimum over the current advices, 3 when none);
  - the layouts (all, small, large) and their controls `0x100100`/`0x100101`; the caption `name - personal name`;
  - the HTML rows: colour by mood (0 blinking red, 1 red, 5 green, others dark blue, greyed without flag 2), title
    and body markup, the sixteen icons `1441625n`;
  - the button and message tables, including the mood message `0x426841A1`.
- Correction to advisors.md §12.1.1 (not edited here): `0x0041D9B0` ("AddMessage") removes and `0x0041DA00`
  ("RemoveMessage") appends.
- docs/specs/questions/advisor-window.md: AW1–AW5 (new prefix AW); QUESTIONS.md regenerated.

### Quirks found

- The add/remove helpers are named the wrong way round.
- `SetAllMoods` does not check the city.
- The tutorial filter's answer is ignored.
- `AdviceRemoved` enables two children that the refresh disables at once.
- An unused colour string.
- A greyed mood-0 row keeps `</BLINK>` without its opening tag.
- The large/small switches do not check the panel.

### Coverage

- tools/coverage-map.tsv: `cSC4AdvisorUIAdvisorWinProc` → advisor-window.md, before the `cSC4Advisor*WinProc`
  catch-all.
- Specified bytes 4,482,615 → 4,492,792 (+10,177, 80.3% → 80.5%, `cov_local.py`, on `5c0ff782` + the eight lanes
  before).

### Not covered

- The face proc `cSC4AdvisorUIImageWinProc` (6 functions, 2,114 bytes: the 3D head with its camera, mood models
  and blinking), AW5.
- The list control (AW1).
