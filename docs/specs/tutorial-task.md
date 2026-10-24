# `cSC4TutorialTask`: one tutorial step (behaviour spec)

Scope: the `cSC4TutorialTask` owner as tools/coverage.py assigns it (33 functions, 6,340 bytes on `5c0ff782`;
inventory in §7). The tutorial window builds one task per entry of the Lua table `tutorialtasks`
(tutorial-window.md §3.1) and asks the current task for:
- its texts;
- the hint window;
- the allowed buttons;
- the zoom and camera;
- the highlighted area;
- the start and end actions.

This spec answers TUW5 of tutorial-window.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled. The Lua key names were read from their start-up string
initializers (`0x00B21048`–`0x00B211EC`), the start jump table from `0x00440730`/`0x00440740`, the vtable from
`0x00A862D8`. Written 2026-10-23 (re-tuttask7).

Conventions:
- `T` is the task.
- `L` is its Lua context (`T+0x84`); its slots are as in lua-host.md §5:
  - 6 `GetTop`;
  - 7 `SetTop`;
  - 11 `Pop`;
  - 14 `IsNumber`;
  - 15 `IsString`;
  - 23 `ToNumber`;
  - 24 `ToString`;
  - 31 `PushNumber`;
  - 33 `PushString`;
  - 37 `GetGlobal`;
  - 38 `GetTable`;
  - 64 `GetN`;
  - 72 `IsTable`.
- `V` is the 3D view (`0x00B43DC4`); "`V`'s window" is its slot 3.
- The city is `0x00B43CEC`.

## 1. Identity and fields *[compat]*

0xFC bytes; constructor `0x00440F30`. Two vtables:
- `0x00A862D8` at `+0`;
- `0x00A86344` at `+4`, the reference-count part: its thunks are `0x004410B0` (`QueryInterface`) and `0x004410C0`
  (deleting destructor).

| Offset | Field |
|---|---|
| `+0x0C`, `+0x2C`, `+0x4C` | the instruction, try-again and congratulation messages (`cADVString`) |
| `+0x6C`…`+0x74` | the cell grid: a vector of rows, each a vector of 12-byte cells `{x, z, flag}` (§3) |
| `+0x7C` | the task's index in `tutorialtasks` |
| `+0x80` | byte: the reads are valid |
| `+0x84` | `L` (borrowed) |
| `+0x88` | the highlight effect name (`cRZString`, default `selection_yellow`) |
| `+0x9C` | byte: started |
| `+0xA0` | the type (`function_index`) |
| `+0xA4`…`+0xB0` | the area, a copy of the four values below |
| `+0xB4`…`+0xC0` | `startX`, `startZ`, `endX`, `endZ` (cells) |
| `+0xC4` | byte: `csi` > 0 |
| `+0xC5` | byte: the CSI visibility saved at start |
| `+0xC8`, `+0xCC` | `cameraX`, `cameraZ` |
| `+0xD0`, `+0xD4` | `window_offsetX`, `window_offsetY` |
| `+0xD8` | `extraData` |
| `+0xDC` | the highlight visual effect (counted) |
| `+0xE0`…`+0xE8` | the allowed button IDs (`std::vector<uint32>`) |
| `+0xEC` | `target_guid` |
| `+0xF0` | byte: the area is taller than wide |
| `+0xF4` | `zoomLevel` |
| `+0xF8` | byte: `no_edge_scroll` > 0 |

Every number defaults to −1 for the area and to 0 elsewhere.

**The interface** (`0x00A862D8`):

| Slot | Address | Behaviour |
|---|---|---|
| 0 | `0x0043FBA0` | `QueryInterface`: IID `0x8A09F5F5` gives `T` with an `AddRef`; any other IID goes to `cRZUnknown` |
| 1, 2 | shared | `AddRef` / `Release` of the `+4` part |
| 3 | `0x00441360` | `Init(L, index)` (§2) |
| 4 | shared | true |
| 5 | `0x0043FD40` | the area (`T+0xA4`) |
| 6 | shared | `T+0x78` (always 0) |
| 7 | `0x0043FF10` | `(id)`: the ID is in the allowed button list |
| 8 | shared | the instruction message (`T+0x0C`) |
| 9 | `0x0043FD30` | the try-again message (`T+0x2C`) |
| 10 | shared | the congratulation message (`T+0x4C`) |
| 11 | shared | `target_guid` |
| 12 | `0x0043FF60` | `GetTargetWindow` (§4) |
| 13 | shared | `(x)`: true |
| 14 | `0x004405F0` | `Start` (§5) |
| 15 | `0x0043FD70` | for types 1, 2 and 0x11, `HighlightRect(area)` again |
| 16 | `0x00440220` | `End` (§5) |
| 17 | `0x00440760` | `HighlightRect(l, t, r, b)` (§5) |
| 18 | shared | `extraData` |
| 19 | `0x004405D0` | the first allowed button ID, or 0 |
| 20 | `0x0043FD20` | the type |
| 21 | `0x0043FD50` | `(id)`: `target_guid` is not 0 and equals `id` |
| 22 | shared | `zoomLevel` |
| 23, 24 | shared | `window_offsetX`, `window_offsetY` |
| 25 | `0x004410A0` | edge scrolling allowed: `no_edge_scroll` is 0 |
| 26 | `0x00440A30` | `TriggerFire(&area)` (§5) |

The tutorial window's slot uses (tutorial-window.md §8) map onto these:
- 8 is the instruction;
- 9 is the try-again message shown after event 1;
- 11 is the hint window ID;
- 12 is the hint window;
- 19 is the catalog item ID;
- 21 is the window-message filter.

**Destructor** (`0x004410D0`, deleting `0x00441340`): free the button list; release the effect; destroy the strings
and the cell grid; `cRZUnknown`'s destructor; `GlobalHeapFree` when bit 0 of its argument is set.

## 2. Reading the task: `Init(L, index)` (`0x00441360`, slot 3)

1. `T+0x84` := `L`; `T+0x7C` := `index`.
2. Remember the stack top (`GetTop`). `GetGlobal("tutorialtasks")`.
3. valid := `GetN(-1)` ≥ `index`.
4. When valid and `index` ≠ 0: `PushNumber(index)`, `GetTable(-2)`; valid := `IsTable(-1)`. With `index` 0 the
   keys below are read from `tutorialtasks` itself *[quirk]*.
5. Read, from the table on top of the stack:
   1. The three messages `instruction_msg`, `try_again_msg` and `congratulation_msg`, when present (§2.1).
   2. The numbers `zoomLevel`, `cameraX`, `cameraZ`, `window_offsetX`, `window_offsetY`, `no_edge_scroll`
      (stored as > 0) and `csi` (stored as > 0).
   3. The string `decal_color` (default `selection_yellow`).
   4. `task_action` (§2.2).
   5. `target_buttons` (§2.3).
6. Restore the stack top (`SetTop`). valid := true.
7. For types 2 and 0x11, build the cell grid (§3). Return true.

**The readers**:
- **Number** (`0x0043FC70`): 0 when not valid. Otherwise:
  1. `PushString(key)`, `GetTable(-2)`.
  2. When the value is not a number: valid := **false**, the result is 0, and the value is left on the stack
     *[quirk]*.
  3. Otherwise the result is the number truncated to an integer; `Pop(1)`.

  So a missing number disables every later read of the same `Init`. Each task must define every key read before
  the keys it relies on.
- **String** (`0x0043FBD0`): false when not valid. Otherwise:
  1. Clear the output.
  2. `PushString(key)`, `GetTable(-2)`.
  3. When it is a string, the output gets `ToString(-1)`; `Pop(1)`; true.
  4. Otherwise `Pop(1)`, false. A missing string does not disable the reads.
- **String with a default** (`0x00441170`): the same, but a missing string copies the default. True when valid.

### 2.1 The messages

Each message value is a text key:
1. The `cADVString` gets the key (its slot 30) and loads it (`cSLString::Load(1, 0)`).
2. When the load fails, the message becomes `"***_NOT_IN_TEXT_DATABASE_*** " + key`, converted from code page
   1252 (`ConvertStringEncoding(…, 0x4E4, …)`), and set through the string's slot 14.

### 2.2 `task_action` (`0x00440DD0`)

False when not valid. Otherwise:
1. `PushString("task_action")`, `GetTable(-2)`; valid := it is a table. Without the table: false.
2. type := `function_index`. Type 0: false.
3. `startX`, `startZ`, `endX`, `endZ` go to `+0xB4`…`+0xC0` and are copied to the area `+0xA4`…`+0xB0`.
4. `extraData`; then `target_guid`, a string read as hexadecimal (`RZAtoUint32HexA`; `0x00440B50`).
5. `Pop(1)`; true.

### 2.3 `target_buttons` (`0x00440BE0`)

False when not valid. Otherwise:
1. `PushString("target_buttons")`, `GetTable(-2)`.
2. When it is not a table: `Pop(1)`, valid := false, false.
3. Otherwise read it as an array of strings (`SCLuaHelp::ReadTableAsArray`). Each string is read as hexadecimal
   and appended to the allowed list. `Pop(1)`.
4. For type 4 with a non-zero `target_guid`, insert the GUID at the **front** of the list.
5. True.

## 3. The cell grid (types 2 and 0x11)

Let `w` = `endX − startX` and `h` = `endZ − startZ`. The "taller" byte is `w < h`.

For each row `r` = 0 … `h` (`z` = `startZ + r`), and each column `c` = 0 … `w` (`x` = `startX + c`), the cell
`{x, z, flag}` is appended to row `r`:
- **type 2**: `flag` is 1 at the two ends along the long axis, else 0;
  - in a wide area: `c` = 0 or `c` = `w`;
  - in a tall area: `r` = 0 or `r` = `h`;
- **type 0x11**: `flag` is 0.

The vector helpers `0x00441230` (grow and insert) and `0x00440B20` (copy cells) serve this grid.

## 4. `GetTargetWindow` (`0x0043FF60`, slot 12)

The result is a borrowed window, or null. `notEstablished` := the city's `GetEstablished` (slot 22) is false.

For each allowed button ID in list order (0 skipped):
1. `b` := `V`'s window's child `id` (recursive). When there is none, skip the ID.
2. `b` gets `SetFlag(2, 1)`: every listed button that exists is enabled, even after a result is found.
3. When there is no result yet, walk up from `p` = `b` towards `V`'s window, with `cand` = `b` and `vis` =
   `b.IsVisible()`. While `p` is not `V`'s window:
   1. When `vis` is false, stop: this ID gives no result.
   2. `vis` := `p.IsVisible()`.
   3. When `notEstablished` and `p`'s parent has ID `0x0A78827A`:
      - `p` := `V`'s window's child `0xC991EDA8`;
      - when `cand` has ID `0xA9ED5617`, `0x69B9324A` or `0x4A551A6B`, `cand` := the direct child (slot 34) of
        `p` with the same ID.
   4. Otherwise `p` := `p`'s parent.
4. When the walk ends with `vis` true and `cand` not null, `cand` is the result.

So the hint goes to the first listed button that is visible with all its ancestors. Before the city is
established, three buttons of the panel `0x0A78827A` are taken from the panel `0xC991EDA8` instead *[inferred:
the start-of-game tool bar]*.

## 5. Start, end, highlight and fire

**`Start`** (`0x004405F0`, slot 14). True at once when already started; false for type 0. Otherwise:
1. When `cameraX` and `cameraZ` are both 0, they become the centre of the area:
   `((startX + endX)/2, (startZ + endZ)/2)`.
2. When both are non-zero, move the camera there (`0x004402B0`, below).
3. By type:
   - **5**: the simulator's `HiddenPause` (slot 10), then `HighlightRect(area)`;
   - **9**: `TriggerFire(&area)`, then `HighlightRect(area)`;
   - **1, 2, 3, 8, 0x11**: `HighlightRect(area)`;
   - any other type: nothing.
4. The city situation manager (city slot 49):
   - `+0xC5` := `GetCSIVisible()`;
   - `SetCSIVisible(csi)`.
5. started := 1; true.

**Camera** (`0x004402B0(x, z)`):
1. Only when the city exists and `CellIsInBounds(x, z)` (city slot 99), and the renderer (`0x00B43DD0`) gives a
   camera (slot 16).
2. The target is `(x · s, height, z · s)`, with `s` = the terrain's slot 6 (`0x00B43CF4`) and `height` = the
   terrain's slot 13 at those two coordinates.
3. `SetViewTarget(&target)` (`0x007CD810`), then the renderer's slot 17 `(camera)`.

**`End`** (`0x00440220`, slot 16). Only when started:
1. Restore the CSI visibility when the saved value differs from `csi`.
2. Stop the highlight effect (`Stop(true)`, effect slot 4) and release it.
3. For type 0, return **false** with the started byte left set *[quirk]*.
4. For type 5, the simulator's `HiddenResume` (slot 13).
5. started := 0.

Without a start, `End` returns true.

**`HighlightRect(l, t, r, b)`** (`0x00440760`, slot 17, RET 16). With the city's effects manager (slot 85):
1. Stop (`Stop(false)`) and release the old effect.
2. When the manager `HasVisualEffect(decal_color)` (slot 5) and `CreateVisualEffect(decal_color, &+0xDC)` (slot 7)
   succeeds:
   1. `SetParameter(0xC, {l, t, r + 1, b + 1} as floats, 4)` (effect slot 12);
   2. `Start(false)` (slot 3).
3. True.

**`TriggerFire(&area)`** (`0x00440A30`, slot 26):
1. `FindNearestBuilding(&area)` (`0x00440920`): spiral out from the area's centre, within the area's corners, for
   the first cell whose lot (lot manager `0x00B43D08` `GetLot(x, z, false)`) has a building (lot slot 48). The
   building is borrowed.
2. With a building, take the disaster layer's (city slot 72) fire manager (`GetDisasterManager(0x621CDA33)`) as
   interface `0x49B43F57`.
3. `IgniteOccupant(building's occupant)` (slot 4), then `SetCause(6)` (slot 9).
4. True; false when any step finds nothing. The manager is used without a null check *[quirk]*.

## 6. Quirks and cases

Quirks:
- A missing number key disables the rest of `Init`'s reads and leaves a value on the Lua stack. The stack is
  restored at the end, but the reads stay lost.
- A missing `target_buttons` table also disables the reads; `Init` then marks the task valid anyway.
- Index 0 reads its keys from `tutorialtasks` itself.
- `End` of a type-0 task returns false and keeps the started byte.
- A camera target with cell 0 on either axis is not moved to.
- `GetTargetWindow` enables every listed button that exists.

| Case | Setup | Expected |
|---|---|---|
| TT-C1 | `task_action = {function_index = 2, startX = 10, startZ = 4, endX = 14, endZ = 4}` | `w` = 4, `h` = 0: one row of five cells; flags 1, 0, 0, 0, 1 |
| TT-C2 | the same with `function_index = 0x11` | the same cells, all flags 0 |
| TT-C3 | `target_buttons = {"6A57DA48", "2A57DB84"}`, type 4, `target_guid = "11"` | list `[0x11, 0x6A57DA48, 0x2A57DB84]` |
| TT-C4 | `cameraX` = `cameraZ` = 0, area (10, 4)–(14, 8) | camera target cell (12, 6) |
| TT-C5 | a task without `zoomLevel` | zoom 0, and every later key reads 0 or its default; type 0 |

## 7. Function inventory (owner `cSC4TutorialTask`)

All 33 functions:
- §1: `0x0043FBA0`, `0x00440F30`, `0x004410D0`, `0x00441340`, `0x004410B0`, `0x004410C0`, `0x00440F00` (a string
  holder's deleting destructor), `0x00440840` (`cTutorialString::init` thunk), `0x0043FD20`, `0x0043FD30`,
  `0x0043FD40`, `0x0043FD50`, `0x0043FD70`, `0x0043FF10`, `0x004405D0`, `0x004410A0`;
- §2: `0x00441360`, `0x0043FC70`, `0x0043FBD0`, `0x00441170`, `0x00440DD0`, `0x00440B50`, `0x00440BE0`,
  `0x0043FDB0` (a helper: string slot 29 `(text, strlen(text))`);
- §3: `0x00441230`, `0x00440B20`;
- §4: `0x0043FF60`;
- §5: `0x004405F0`, `0x004402B0`, `0x00440220`, `0x00440760`, `0x00440A30`, `0x00440920`.

Not covered: who reads the cell grid (the click filter and `DoMessage` of the tutorial system, tutorial.md); the
`cADVString` loader.
