# `cSC4AdvisorUIAdvisorWinProc`: the advisor window (behaviour spec)

Scope: the whole `cSC4AdvisorUIAdvisorWinProc` owner as tools/coverage.py assigns it (26 functions, 10,177 bytes on
`5c0ff782`; inventory in §8). Its lifetime calls are owned by the advisor system and are in advisors.md §12.1.1:
- the constructor `0x0041DB50`;
- show `0x0041D650`;
- the teardown `0x0041D830`;
- the five-slot ABI.

This spec gives the bodies behind them. The per-advisor face procs (constructor `0x00420B70`) are specified in
advisor-face.md (slot 6 `SetVisible`, 7 `SetPressedOffset`, 8 `SetMood`) and the HTML list control in
advice-list.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled; the HTML strings and the tables were read from the data
section. Written 2026-10-23 (re-advwin7).

Conventions:
- `W` is the win proc.
- `sys` is the advisor system, reached fresh every time as the city `0x00B43CEC` slot 81 (`+0x144`).
  `GetAdvisor(id)` is its slot 7.
- `P` is the main game window, reached as `W+0x18` slot 3, then slot 11 (the parent).
- Children are found with slot 35, shown and hidden with `SetFlag(1, on)` (slot 68), and enabled and disabled
  with `SetFlag(2, on)`.

## 1. Identity and layout *[compat]*

| Offset | vtable | Slots |
|---|---|---|
| `+0x00` | `0x00A83548` | 0 QI (`0x0041D400`), 1 AddRef (shared), 2 `Release` (`0x0041D460`), 3 `DoWinProcMessage` (`0x0041F830`), 4 `DoWinMsg` (shared), 5 deleting destructor (`0x0041DC00`) |
| `+0x04` | `0x00A83538` | 0 QI thunk (`0x0041DB30`), 1 AddRef (shared), 2 Release thunk (`0x0041DB40`), 3 `DoMessage` (`0x0041FD10`) |

- **QI**:
  - `0x22E85D8E` gives `W`;
  - `0x452294AA` and 1 give `W+4` (null when `W` is null), AddRef'd;
  - other IIDs: false.
- **`Release`**: a count above 1 is decremented. Otherwise the deleting destructor runs with 1, and 0 is
  returned.
- **Destructor body** (`0x0041DA60`), in order:
  1. clear and free the map `+0x64`, then the list `+0x60`;
  2. release `+0x5C`;
  3. release the seven `+0x38` entries, then the seven `+0x1C` entries (each from the last);
  4. release `+0x18` and `+0x0C`.

| Offset | Field |
|---|---|
| `+0x08` | the count |
| `+0x0C` | the advisor shown in the individual window (counted) |
| `+0x10`, `+0x14` | its ID and its index in the ID table, −1 when none |
| `+0x18` | the script window's `cIGZWinGen` (`0x5386D516`) |
| `+0x1C`…`+0x34` | 7 `cIGZWinGen` of the face windows |
| `+0x38`…`+0x50` | 7 face procs (counted) |
| `+0x54` | byte: subscribed |
| `+0x55` | byte: the individual window is the large one |
| `+0x56` | byte: visible |
| `+0x58` | the layout: 0 all advisors, 1 small individual, 2 large individual |
| `+0x5C` | the message list control of the individual window |
| `+0x60` | list of the advices shown (12-byte nodes: next, previous, advice) |
| `+0x64`, `+0x68` | map: a copy of the shown advisor's current advices |

**The tables.** The advisor ID table `0x00A83508`, by index 0–6: `0x19`, `0x15`, `0x16`, `0x17`, `0x18`, `0x1A`,
`0x1B`.

| Index | Advisor | Face window (all-advisors panel) | Button |
|---|---|---|---|
| 0 | `0x19` | `0x0A15C7A1` | `0xAA15C795` |
| 1 | `0x15` | `0x6A15C7B5` | `0x6A15C7AA` |
| 2 | `0x16` | `0x4A15C7C6` | `0x6A15C7BE` |
| 3 | `0x17` | `0x0A15C7D8` | `0xCA15C7CF` |
| 4 | `0x18` | `0x6A15C7EA` | `0xAA15C7E2` |
| 5 | `0x1A` | `0xEA15C7FA` | `0x2A15C7F1` |
| 6 | `0x1B` | `0x8A15C80C` | `0x8A15C802` |

The three layout panels, all children of `P`:
- `0x6A15C767`: all advisors;
- `0xAA15EF06`: small individual;
- `0x2A1D96B1`: large individual.

## 2. `Init` (`0x0041E320`) *[compat]*

1. A temporary set gets `P`'s current children (`nSC4UI::GetWindowChildrenSet`).
2. `+0x18` is released and remade: `SC4CreateWindowFromScript(&{0, 0x96A006B0, 0xCBC905CD}, P, 0x6A15C767,
   0x5386D516, &+0x18)`.
3. `0x0078F3C0(P, &set, &set)` *[inferred: keeps the new children only]*; `nSC4UI::AlignWindowSetToWindow(&set, 0)`.
4. `nSC4UI::ClearAcceptFocusFlagRecursive` runs on the script window and on each of the three layout panels. Each
   panel is also hidden.
5. `+0x18` slot 7 `(W)`: `W` becomes the window's proc.
6. `SetupAllAdvisorWinProcs`; the temporary set is freed.

## 3. Faces and moods *[compat]*

- **`get_advisor_mood(advisor)`** (`0x0041DC20`):
  - 3 when the advisor is null or has no current advice;
  - otherwise the smallest mood (advice slot 6) of its current advices (a copy of slot 11), starting from 7 and
    stopping at 0.
- **`SetupSingleAdvisorWinProc(id, child)`** (`0x0041DCE0`, RET 8):
  1. Nothing for an unknown ID or when its face proc exists.
  2. `w` = `P`'s child `child`. A new face proc (0xF8 bytes, constructor `0x00420B70(id)`) goes in `+0x38[i]`.
  3. `w` gets `SetFlag(0x8000000, 1)`. Its `cIGZWinGen` goes in `+0x1C[i]` (the old one released), and that gets
     slot 7 `(proc)`.
  4. The proc gets slot 6 `(+0x56)`.
  5. With a city: the proc's slot 8 `(get_advisor_mood(GetAdvisor(id)))`.
- **`SetupAllAdvisorWinProcs`** (`0x0041DE20`): the same for every index without a face proc, with the face
  windows of the table.
- **`SetAllMoods`** (`0x0041DF90`): each existing face proc gets slot 8 `(get_advisor_mood(GetAdvisor(id)))`. The
  city is not checked *[quirk]*.
- **`RemoveSingleAdvisorWinProc(id)`** (`0x0041D4D0`): for a known ID, `+0x1C[i]` slot 7 `(0)`, then `+0x38[i]` is
  released and nulled.
- **`0x0041D530`** (all seven): when `+0x38[i]` exists:
  1. `+0x1C[i]` slot 7 `(0)`;
  2. `+0x38[i]` slot 6 `(0)`;
  3. release and null.

## 4. Layouts *[compat]*

- **`SetAllAdvisorsWindow`** (`0x0041E1C0`):
  1. Hide the small and large panels; show the all-advisors panel.
  2. `RemoveSingleAdvisorWinProc(+0x10)`; release `+0x0C`.
  3. `+0x10` = `+0x14` := −1.
  4. `SetupAllAdvisorWinProcs`; `+0x58` := 0.
- **`SetIndividualAdvisorWindow(id)`** (`0x0041F2D0`, RET 4); nothing for an unknown ID:
  1. **The advisor.** All face procs are dropped (`0x0041D530`). `+0x0C` := `GetAdvisor(id)` (counted),
     `+0x10` := `id`, `+0x14` := its index. The all-advisors panel is hidden.
  2. **The panel**, with the list control fetched into `+0x5C` only when that is null:
     - **`+0x55` set:** the large panel. The control is the panel's child `0x100101` (IID `0xCA1492A2`, slot 37).
       `+0x58` := 2. The face window is `0xAA1D97DF`.
     - **otherwise:** the small panel. The control is child `0x100100`. `+0x58` := 1. The face window is
       `0xEA15EFC7`.

     Then `SetupSingleAdvisorWinProc(id, face window)`, and the panel is shown.
  3. **The caption** = `name + " - " + personal name` (advisor slots 6 and 8). It goes to children `0x2A15F5E8` and
     `0xEA1D980D` of `P` and to the panel (slot 74).
  4. `GetMessageList(+0x0C)`; `updateBodyTextCtrl`.
- **`SetLargeIndividualAdvisorWindow`** (`0x0041F610`) and **`SetSmallIndividualAdvisorWindow`** (`0x0041F720`).
  Both do nothing unless an advisor is shown (`+0x14` < 7). Otherwise:
  1. Hide the other panel and show this one.
  2. The old control, when set, is cleared (slot 7), released and fetched again from this panel (`0x100101`
     large, `0x100100` small). The panel is not checked for null.
  3. Large: `+0x58` := 2, `+0x55` := 1. Small: `+0x58` := 1, `+0x55` := 0.
  4. `RemoveSingleAdvisorWinProc(+0x10)`, then `SetupSingleAdvisorWinProc(+0x10, 0xAA1D97DF / 0xEA15EFC7)`.
  5. `GetMessageList(GetAdvisor(+0x10))`; `updateBodyTextCtrl`.
- **`HideWindow`** (`0x0041D710`):
  1. Hide the three panels.
  2. With an individual advisor (`+0x14` < 7): its face proc, when it exists, gets slot 6 `(0)`.
  3. Otherwise all seven face procs get slot 6 `(0)`.
  4. `+0x56` := 0.

## 5. The advice list

### 5.1 Maintenance *[compat]*

- **`GetMessageList(advisor)`** (`0x0041E000`, RET 4):
  1. Each listed advice without flag 2 (slot 15) gets `SetFlags(0x20, false)` (slot 14).
  2. The map `+0x64` := a copy of the advisor's current advices (slot 10).
  3. The list is emptied, then refilled from the map in key order (counted).
  4. `SetAllMoods`.
- **`0x0041D9B0`** (named `AddMessage`) **removes** the first node holding the advice and releases it.
  **`0x0041DA00`** (named `RemoveMessage`) **appends** the advice, counted. advisors.md §12.1.1 calls
  `0x0041D9B0` an add; the bodies are the other way round.
- **`RegisterMessageNotification(on)`** (`0x0041D580`, RET 4):
  - on, when not subscribed: `+0x54` := 1, then `W+4` is subscribed (server `0x00B43CCC` slot 5) to the five
    messages of §6.2;
  - off, when subscribed: `+0x54` := 0, then unsubscribed (slot 6).

  The byte changes even when there is no server.

### 5.2 `updateBodyTextCtrl` (`0x0041E4B0`)

Static HTML strings are built once each, guarded by the bits of `0x00B20B94`:

| String | Text |
|---|---|
| colours | `<FONT COLOR="#8e9499"` (grey), `#3f4967` (dark blue), `#095a22` (green), `#b00000` (red), `<BLINK><FONT COLOR="#f70a0b"` (blinking red), `#0f0f0f` (unused) |
| title face | ` FACE="StocletITC TT" SIZE=3><B>` |
| title close | `</B></FONT>`, `</B></FONT></BLINK>` |
| body open | `<FONT COLOR="#3f4967" FACE="Arta" SIZE=3><I>`, grey `#8e9499` variant |
| body close | `</I></FONT>` |
| icons | `<IMG SRC="sc4://HTML/46a006b0/1441625n">` for `n` = 0…F |

Nothing more without a control `+0x5C`. Otherwise:

1. The control (a `cSC4WinAdviceList`, advice-list.md) is cleared (slot 7) and gets the background `0x14416264`
   (slot 6).
2. For each listed advice, by its mood `m` (slot 6):

   | Mood | Colour | Title close | Icons (`a`, `b`, `c`, `d`) |
   |---|---|---|---|
   | 0 | blinking red | `</B></FONT></BLINK>` | 0–3 |
   | 1 | red | `</B></FONT>` | 4–7 |
   | 5 | green | `</B></FONT>` | C–F |
   | other | dark blue | `</B></FONT>` | 8–B |

   - An advice without flag 2 is greyed: grey colour, icons 8–B, and the grey body open.
   - The title = `colour + title face + title text + title close`, the text from advice slot 18 and the string's
     slot 5.
   - The body = `body open + message text + </I></FONT>`, the text from slot 20.
   - The control's slot 4 (`AddEntry`: open icon, closed icon, body, title, open, advice, close icon, static,
     background) gets the row:
     - mood 7: `(a, a, &body, &title, 0, advice, &d, 0, 0x14416264)`;
     - otherwise: `(c, b, &body, &title, CheckFlags(0x20), advice, &d, 0, 0x14416264)`.
3. The control's slot 5 `(0)`: `RebuildHTML` without keeping the scroll position (advice-list.md §3; AW1,
   answered).
4. Children `0xAA1D969D` and `0x6A1D96C3` of `P` are disabled (`SetFlag(2, 0)`), even with an empty list.

## 6. Messages *[compat]*

### 6.1 `DoWinProcMessage(w, m)` (`0x0041F830`, RET 8)

False unless `w` is the script window (`+0x18` slot 3). The tutorial filter (`FilterUICommandMessage`) runs, and
its answer is ignored *[quirk]*. Every path returns false.

| `m[0]` | Effect |
|---|---|
| `0xA2BF8AD5` | `RegisterMessageNotification(1)`; 22 children of `P` get the script window as notification target (slot 86; not null-checked): `0xCA19DD28`, `0x2A84B45A`, `0x8A15EFE6`, `0x0A15EFBC`, `0xAA1D9734`, `0x2A84B45B`, `0x2A1D973E`, `0x6A1D97D5`, `0x4A15EFDE`, `0xCA1D974A`, the seven buttons, `0xAA1D969D`, `0x6A1D96C3`, `0x6A19D8F2`, `0xAA19DC53`, `0x0A1D96CC` |
| `0xA2BF8AD6` | `RegisterMessageNotification(0)` |
| 3, `m[1]` = `0x287259F6` (button) | by `m[2]`, below |
| 3, `m[1]` = `0x287259F8` | an advisor button: its face proc, when it exists, gets slot 7 `(m[3])` |

The buttons (`m[2]` with `m[1]` = `0x287259F6`):

| Button | Effect |
|---|---|
| `0x0A1D96CC`, `0x6A19D8F2`, `0xAA19DC53` | `HideWindow` |
| the seven advisor buttons | `SetIndividualAdvisorWindow(the table's advisor)` |
| `0x6A1D97D5`, `0x0A15EFBC`, `0x2A1D973E`, `0x8A15EFE6` | `SetAllAdvisorsWindow` |
| `0x2A84B45A`, `0xCA19DD28` | `SetLargeIndividualAdvisorWindow` |
| `0x2A84B45B`, `0xAA1D9734` | `SetSmallIndividualAdvisorWindow` |

### 6.2 `DoMessage(m)` (`0x0041FD10`, `+4` slot 3)

Always true. `adv` is the message's object (slot 32) asked for `cISC4Advice` (`0x8A09F5F5`). It counts only when
its advisor ID (slot 11) is the shown one (`+0x10`).

| Message (slot 4) | Effect |
|---|---|
| `0x8A09FA64` AdviceAdded | a matching `adv` is appended (`0x0041DA00`) |
| `0xCA09FA6C` | a matching `adv` is removed (`0x0041D9B0`). The advice list sends it when the player closes a row (advice-list.md §4; AW2, answered) |
| `0x8A09FA65` AdviceRemoved, `0x8A09FA67` AdviceRemovedAll | a matching `adv` enables `0xAA1D969D` and `0x6A1D96C3` (`SetFlag(2, 1)`) |
| `0x426841A1` | `id` = data1 (slot 9); for `0x15`…`0x1B`, that advisor's face proc, when it exists, gets slot 8 `(data2)` (slot 14). Nothing more. The advice list posts it when a row is toggled, with the advice's advisor ID and its slot 6 (the mood) (advice-list.md §4; AW3, answered) |

After the first three rows, `SetAllMoods` and `updateBodyTextCtrl` run.

Notes on these rows:
- `AdviceRemovedAll` carries the advisor, which is not an advice, so only the refresh happens.
- The enabling in the third row is undone at once: `updateBodyTextCtrl` ends by disabling both children
  *[quirk]*.
- `AdviceRemoved` does not take the advice out of the list. It stays until the next `GetMessageList`.

## 7. Quirks and cases

Quirks:
- `0x0041D9B0` and `0x0041DA00` do the opposite of their names.
- `SetAllMoods` does not check the city.
- The tutorial filter's answer is ignored.
- The two children enabled by `AdviceRemoved` are disabled again by the refresh.
- The `#0f0f0f` colour string is built and never used.
- A greyed mood-0 advice keeps the closing `</BLINK>` without the opening one.
- `SetLarge…`/`SetSmall…` do not check the panel.

| Case | Setup | Expected |
|---|---|---|
| AW-C1 | advisor with advices of moods 4, 2, 6 | `get_advisor_mood` = 2 |
| AW-C2 | advisor with no advice | 3 |
| AW-C3 | button `0xCA15C7CF` | `SetIndividualAdvisorWindow(0x17)`, index 3 |
| AW-C4 | advice of mood 0 with flag 2 | blinking red title ending `</B></FONT></BLINK>`, icons `14416252` and `14416251` (`c`, `b`), `d` = `14416253` |
| AW-C5 | the same without flag 2 | grey title still ending `</B></FONT></BLINK>`, icons `1441625A`, `14416259`, `d` = `1441625B`, grey body |
| AW-C6 | message `0x426841A1` with data1 `0x19`, data2 4 | face proc 0 gets slot 8 `(4)` |

## 8. Function inventory (owner `cSC4AdvisorUIAdvisorWinProc`)

All 26 functions (10,177 bytes):
- §1: `0x0041D400`, `0x0041D460`, `0x0041DA60`, `0x0041DB30`, `0x0041DB40`, `0x0041DC00`;
- §2: `0x0041E320`;
- §3: `0x0041DC20`, `0x0041DCE0`, `0x0041DE20`, `0x0041DF90`, `0x0041D4D0`, `0x0041D530`;
- §4: `0x0041E1C0`, `0x0041F2D0`, `0x0041F610`, `0x0041F720`, `0x0041D710`;
- §5: `0x0041E000`, `0x0041D9B0`, `0x0041DA00`, `0x0041D580`, `0x0041E4B0`, `0x0041E270` (`string + const char*`,
  the HTML builder's helper);
- §6: `0x0041F830`, `0x0041FD10`.
