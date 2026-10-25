# `cSC4BuildingSelectWinProc`: the building styles panel (behaviour spec)

Scope: the `cSC4BuildingSelectWinProc` owner as tools/coverage.py assigns it (13 functions, 2,178 bytes). The panel
is the mayor-mode window where the player picks the architectural styles that grow (style IDs `0x2000`–`0x2003`:
Chicago, New York, Houston, Euro), chooses between "change styles every N years" and "use all styles at once", and
sets N. All state lives in the tract developer (tract-developer.md §4.20); the panel only shows and edits it. The 3D
view owns the proc (view3d-window.md: window `0x6BC61F19`, proc at `+0x2FC`).

Related: tract-developer.md §4.20 and §7.1 (`cISC4TractDeveloper` slots 9–14, global `0x00B43D24`),
building-development.md (the style occupant groups), widget-button.md (`nsGZWinUtils` toggle helpers), ui-helpers.md,
tutorial-window.md (`FilterUICommandMessage`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile; `Init`, `ShowWindow` and the style
edits were checked on the instructions. Written 2026-10-24 (re-gzstyles7).

`T` is the tract developer (`0x00B43D24`).

## 1. The proc *[compat]*

| Offset | Field | Constructor (`0x00769750`) |
|---|---|---|
| `+0x00` | vtable | |
| `+0x04` | reference count | 0 |
| `+0x08` | initialised (byte) | 0 |
| `+0x0C` | the panel window (constructor argument, counted) | |
| `+0x10` / `+0x14` | the two pages (children `0x8BCA20C3` and `0x2BC619AF`, counted) | null |
| `+0x18` | second page showing (byte) | 1 |

- **`QueryInterface`** (`0x007693E0`): only `0x22E85D8E` → the proc (with a reference); everything else, IID 1
  included, false.
- **Destructor** (`0x00769790`): `Shutdown`, then the window and both pages are released.

## 2. Lifetime *[compat]*

**`Init`** (`0x00769420`), the first time:

1. The pages are taken (window slot 34); the first is hidden (slot 70), the second shown (slot 69).
2. The proc becomes the window's proc (the window as `0x5386D516`, generic-window slot 7).
3. Flag `0x800` (z-order movable) is cleared on the window and both pages.
4. When the radio button `0xCBC61559` exists (child as button `0x8810`, slot 37): it is sized to fit (button slot 33);
   then, when the windows `0xCBC3C2B9` (A) and `0xCBC61581` (B) exist: `A.MoveTo(A.GetT(), buttonWindow.GetR() + 2)`
   and `B.MoveTo(B.GetT(), A.GetR() + 8)` — x and y **swapped** *[quirk]*: the evident intent is to place A right of
   the button and B right of A.
5. Initialised := 1.

Every call ends with `SetWindowNotificationTargetRecursive(window, window)` and returns true.

**`Shutdown`** (`0x007695F0`), when initialised: clears the flag, removes the proc from the window (generic slot 7,
null) and releases both pages. True.

**`ShowWindow`** (`0x00769B30`) refreshes every control from `T`, then shows the window (slot 69):

- toggle `0xEBC61560` := `T.IsUsingAllStylesAtOnce()` (slot 11); toggle `0xCBC61559` := its negation;
- each style button `0x2000`…`0x2003` := whether that ID is in `T.GetActiveStyles()` (slot 9, copied);
- `EnableStyleButtons` (§3);
- the caption of `0xABC61589` := `"%d"` of `T.GetYearsBetweenStyleChanges()` (slot 13), and the spinner `0xABC61550`
  (child as `0x612CE0C3`, its slot 9) gets the same number (helper `0x009AFF88`, true when the spinner exists).

**`HideWindow`** (`0x007696B0`): window slot 70.

## 3. Commands and edits *[compat]*

**`DoWinProcMessage(win, msg)`** (`0x00769920`) always returns false, except that a command the tutorial filter
rejects (`nsSC4TutorialSystem::FilterUICommandMessage`) returns true at once.

Commands (type 3, `0x287259F6`, button ID at `+8`):

| Button | Action |
|---|---|
| `0xCBC61559` | `T.UseAllStylesAtOnce(false)` (slot 12); toggles `0xCBC61559` on, `0xEBC61560` off |
| `0xEBC61560` | `T.UseAllStylesAtOnce(true)`; toggles `0xEBC61560` on, `0xCBC61559` off |
| `0x0BC61548`, `0x2BC619F3` | flip pages: when the second page is not showing, hide the first and show the second (flag := 1); else the reverse (flag := 0) (`0x00769660`) |
| `0xEBC619FD` | hide the first page, show the second, flag := 1 |
| `0xCBC61567` | hide the second page, show the first, flag := 0 |
| `0x2000`…`0x2003` | the button's state (`+0x0C`) on → add the style, off → remove it; then `EnableStyleButtons` |

The spinner's change message (type `0x682876C9`, sender `0xABC61550`): `T.SetYearsBetweenStyleChanges(value)`
(slot 14, value at `+8`); then the caption of `0xABC61589` is rewritten from `T`'s getter.

- **Add style** (`0x007697F0`): a copy of the active styles; when the ID is missing it is appended and
  `T.SetActiveStyles(copy)` (slot 10). An ID already present changes nothing.
- **Remove style** (`0x00769890`): a copy; when the ID is present it is erased and `T.SetActiveStyles(copy)`.
- **`EnableStyleButtons`** (`0x007696D0`): when more than one of the four style buttons is on, all four are enabled;
  otherwise every button that is on (the last one) is disabled, so the active set can never become empty from the
  panel. With none on, nothing is disabled.

## 4. Quirks

- `Init` swaps x and y when it places the two windows next to the "every N years" radio button (§2 step 4).
- `QueryInterface` does not answer IID 1.
- Adding or removing a style goes through a full copy of the active list each time; `SetActiveStyles` keeps the
  current style when it is still present (tract-developer.md §4.20).

## 5. Function inventory

`0x007693E0` `QueryInterface`, `0x00769420` `Init`, `0x007695F0` `Shutdown`, `0x00769660` page flip, `0x007696B0`
`HideWindow`, `0x007696D0` `EnableStyleButtons`, `0x00769750` constructor, `0x00769790` destructor, `0x007697F0` add
style, `0x00769890` remove style, `0x00769920` `DoWinProcMessage`, `0x00769B30` `ShowWindow`, `0x009AFF88` set a
spinner's value by ID.
