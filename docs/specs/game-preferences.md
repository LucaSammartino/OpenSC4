# `cSC4GamePreferencesWinProc`: the game preferences dialog (behaviour spec)

Scope: the `cSC4GamePreferencesWinProc` owner as tools/coverage.py assigns it (8 functions; inventory in §5). The
dialog has:
- five play-option checkboxes stored in the preferences block;
- Accept, Cancel and Credits buttons;
- two pages;
- two buttons of the surrounding menu, one of which resets the tutorial cities.

The credits dialog's constructor (`0x0076A060`) is assigned here by reach; the rest of `cSC4CreditsWinProc` is its
own owner.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled; the dialog table (`0x00AB4B54`) and the vtable
(`0x00AB4B64`) were read from the data section. Written 2026-10-23 (re-gameprefs7).

Conventions:
- `G` is the window procedure object.
- `D` is its script window (`G+4`, a `cIGZWinGen`; "`D`'s window" is its slot 3).
- `P` is the preferences block: `G+8`, taken from the global `0x00B43CD8` when null.
- "Child `id`" is `GetChildWindowFromIDRecursive` (slot 35) on `D`'s window.
- A checkbox is reached as the button interface `0x8810` through `GetChildAsRecursive` (slot 37). Its slot 6
  checks it, slot 7 unchecks it, slot 8 reads it.

## 1. Identity and fields *[compat]*

16 bytes:

| Offset | Field |
|---|---|
| `+0x0` | vtable `0x00AB4B64` |
| `+0x4` | `D` (counted) |
| `+0x8` | `P` (borrowed) |
| `+0xC` | the reference count |

The vtable:
- 0 `QueryInterface` (shared);
- 1 `AddRef` (shared);
- 2 `Release` (`0x0076B500`);
- 3 `DoWinProcMessage` (`0x0076B540`);
- 4 `DoWinMsg` (shared);
- 5 `ShowDialog(kind)` (`0x0076AC30`);
- 6 `Shutdown` (`0x0076AAA0`, `P` := null, true).

The constructor (`0x0076AB30`) zeroes the fields.

**`Release`**: a count above 1 is decremented and returned. At 1 or 0:
1. `Shutdown`;
2. reset the vtable;
3. release `D`;
4. free the object (global heap);
5. return 0.

Two static bytes:
- `0x00B4D970`: the dialog is open;
- `0x00B4D971`: the first `ShowDialog` has run.

**The checkboxes**:

| Checkbox | Preference byte | Meaning (other specs) |
|---|---|---|
| `0x4A7F19F2` | `P+0xF08` | follow disasters (disasters.md) |
| `0x4A7F19F3` | `P+0xF09` | edge scrolling (application.md §13.2) |
| `0x4A7F19F4` | `P+0x4EC` | ask about terrain edge reconciliation (foundations.md §11.6) |
| `0x4A7F19F5` | `P+0xF0B` | urgent advice dialogs (advisors.md) |
| `0x4A7F19F6` | `P+0xF0A` | expanded tooltips (application.md §13.2) |

## 2. `ShowDialog(kind)` (`0x0076AC30`, slot 5)

1. The first time, set `0x00B4D971` and clear the open byte. Later, when the dialog is already open, return true.
2. Look `kind` up in the one-entry table `0x00AB4B54` = `{0, 0x0A7DF315, 0x2A57DB82}` (kind, script instance,
   window ID). Any other `kind`: false.
3. Release `D`. Create the window: `SC4CreateWindowFromScript` (`0x005F9480`) with:
   - the key `{0, 0x96A006B0, 0x0A7DF315}`;
   - the parent 0;
   - the ID `0x2A57DB82`;
   - the IID `0x5386D516`;
   - the result going into `&D`.

   When that fails: false.
4. Size `D`'s window to its parent's width and height (slots 11, 41, 42, then `SetW`/`SetH`). Without a parent,
   use 640 × 480.
5. Show the first page: child `0x2A57DC83` gets `SetFlag(1, 0)` and child `0x2A57DB84` gets `SetFlag(1, 1)`.
6. When `P` exists (after the fallback to `0x00B43CD8`):
   1. Set each checkbox from its byte (slot 6 when the byte is not 0, else slot 7).
   2. `0x6A57DA48`, `0xEA57DA59` and `0x0A57DA65` get `D`'s window as their notification target (slot 86).
   3. Button `0xEA5E99D9`: when a city exists (`0x00B43CEC`), it is disabled (`SetFlag(2, 0)`). Otherwise it gets
      the notification target.
   4. `0xEA57DB7E` and `0xEA57DB7F`, looked up in `D`'s **parent** window, get the notification target.
7. Open byte := 1. `D` gets `G` as its window procedure (generator slot 7).
8. Return **false**, even on success *[quirk]*.

**`DoModalWin`** (`0x0076AB50`):
1. Hold a `cSC4AutoGamePause(0, 0, 1, 1)`.
2. Remember the focus window (window manager slot 36).
3. `D`'s window, held for the call:
   1. the location store `0x00B43CE0` slot 10 `(w, 1)`;
   2. `SetFocus(w)` (manager slot 37);
   3. `DoModalWin(w)` (manager slot 41);
   4. the location store slot 6 `(w)`.
4. Give the focus back when the old window is still valid (slot 24).
5. Close (below).
6. True.

**Close** (`0x0076AAB0`). Only while the open byte is set:
1. Hold `G`.
2. With `D`, and its window `w`, when `w`'s window manager exists:
   1. when `w` has flag `0x1000` (`GetFlag`), `GZWinMsgPost(0xF, w, 1)` ends the modal loop;
   2. `DestroyWindow(w)` (manager slot 23).
3. Release `D` and clear it.
4. Open byte := 0.
5. Release `G` (tail call).

## 3. Messages

**`DoWinProcMessage(win, msg)`** (`0x0076B540`):
- False unless `win` is `D`'s window and it is visible (slot 71).
- Messages of another kind than 3: false.
- Kind 3: true. When the command is `0x287259F6` (a button) and the button is one of `0xEA57DA59`, `0x0A57DA65`,
  `0x6A57DA48`, `0xEA57DB7E`, `0xEA57DB7F` or `0xEA5E99D9`, `HandleControlMessage(button, win)` runs first.

**`HandleControlMessage(button, win)`** (`0x0076B110`, always true):

| Button | Effect |
|---|---|
| `0xEA57DA59` (accept) | when `P` exists, each checkbox's state (slot 8) is written to its byte (1 or 0). Then, always: post `0x231BBF91` PreferencesChanged (message server slot 4); `SavePreferences` (`cISC4App` slot 17); close |
| `0x6A57DA48` (cancel) | close, nothing saved |
| `0x0A57DA65` (credits) | remember the focus; construct a `cSC4CreditsWinProc` (16 bytes, `0x0076A060`: its vtable `0x00AB4AFC`, fields 0), `AddRef`, `ShowDialog(0)` (`0x0076A2C0`), `DoModalWin` (`0x00769F70`); give the focus back when still valid; release it |
| `0xEA57DB7E` | page 1: `0x2A57DC83` hidden, `0x2A57DB84` shown |
| `0xEA5E99D9` | page 2: `0x2A57DC83` shown, `0x2A57DB84` hidden |
| `0xEA57DB7F` | when the application exists: `SC4SendMessage2(0xAC3B3FA6, 0, 0, 0, 0, 0)`, which makes the nation reset the tutorial cities (nation.md); then close |

So the tutorial-city reset (`0xEA57DB7F`) and the second page (`0xEA5E99D9`) come from the surrounding menu, and
the second page's button is disabled inside a city.

## 4. Quirks and cases

Quirks:
- `ShowDialog` returns false on success.
- Accept posts PreferencesChanged and saves even when there is no preferences block.
- Cancel changes nothing, but the checkboxes keep their on-screen state until the window is destroyed.

| Case | Setup | Expected |
|---|---|---|
| GP-C1 | `P+0xF09` = 1, others 0; open | only `0x4A7F19F3` checked |
| GP-C2 | uncheck `0x4A7F19F3`, Accept | `P+0xF09` = 0; `0x231BBF91` posted; preferences saved; dialog closed |
| GP-C3 | `ShowDialog(1)` | false, no window |
| GP-C4 | in a city | `0xEA5E99D9` disabled |

## 5. Function inventory (owner `cSC4GamePreferencesWinProc`)

All 8 functions:
- §1: `0x0076AB30`, `0x0076B500`;
- §2: `0x0076AC30`, `0x0076AB50`, `0x0076AAB0`;
- §3: `0x0076B540`, `0x0076B110`, `0x0076A060` (the credits constructor).

Not covered: `cSC4CreditsWinProc` (its own owner); the location store `0x00B43CE0` (ui-helpers.md UIH1).
