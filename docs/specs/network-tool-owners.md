# Network tool owners: the network tool control, the intersection tool, the toll booth tool, the network preview (behaviour spec)

Scope: the bodies of five owners that the network specs leave open, as tools/coverage.py assigns them:
`cSC4ViewInputControlNetworkTool` (the view-input control that hosts a network tool and its UI front end),
`cSC4ViewInputControlNetworkIntxTool` (the highway intersection tool), `cSC4ViewInputControlTollBooth` (the toll
booth preview tool), `cSC4NetworkPreview` (the cell quads a network tool draws while drafting) and
`cISC4NetworkTool::tPlacementParams` (its copy operator). The UI front end is network-tool-ui.md; the network tool
itself is network-tools.md; the intersection rules and their rings are network-rules.md §10; the base view-input
control is city-tools-input.md §2.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly, the decompile only as a guide. Constants and
tables were read in the PE. Written 2026-10-25 (re-nettools8).

Names used below. "The view" is the held 3D view `+0x1C` (`cISC4View3DWin`, ui-windows.md §3.3): slot 35
`KillKeyboardScrolling`, slot 36 `PickTerrain(x, z, &pos, 0)`, slot 40 `SetCursorText(key, 0, 0, &text, 0)`, slot 41
`ClearCursorText(key)`, slot 43 `SetCostIndicator(cost)`, slot 44 `ClearCostIndicator(0)`. "The terrain" is
`0x00B43CF4` (`cISTETerrain`): slot 6 cell width, 7/8 cell counts, 13 `GetAltitude(x, z)`, 14
`GetAltitudeAtVertex`, 33 `GetWaterDepthAtVertex`, 66 `GetView` (the terrain view; its slot 20 is the state that
hides the local grid, city-tools-input.md §3.4), 70 `IsTriangulationFlipped(x, z)`, 83 the water-surface display
state. An effect's slots are effects.md's table: 3 `Start(0)`, 4 `Stop(0)`, 5 `IsStarted`, 7 set the secondary
transform, 8 `SetTransform`, 15 get the secondary transform. "Snap to the cell centre" with cell width w means
`c := floor(c / w)·w + w·0.5` (floor `0x009EFF60`, the 0.5 a double at `0x00A92D28`), for x and z; y is kept.
A transform descriptor is the 56-byte record of local-grid-helper.md §3.3 (two flag bytes, a 3×3 matrix, the
translation, the scale).

---

## 1. The network tool control (`cSC4ViewInputControlNetworkTool`) *[compat]*

### 1.1 Identity and state

Primary vtable `0x00AAB008` (the 30 view-input slots; overridden: 3 `Init` `0x00662160`, 4 `Shutdown`
`0x00661DC0`, 14 `OnKeyDown` `0x00661E90`, 16 `OnMouseDownL` `0x00661EE0`, 18 `OnMouseUpL` `0x00661F50`, 20
`OnMouseMove` `0x00662290`, 22 `OnMouseExit` `0x00661FC0`, 26 the deleting destructor `0x00662100`). A message
target sub-object at `+0x28` (vtable `0x00AAAFF8`, slot 3 `DoMessage` `0x00662010`). The 3D view's network
commands each create one with `GlobalHeapAlloc(0x58)` and the network type as the argument (`0x007EA1C8…0x007EA35C`,
view3d-window.md §6g).

| Offset | Meaning | Constructor (`0x006620A0`) |
|---|---|---|
| `+0x2C` | the terrain view (terrain slot 66), held | null |
| `+0x30` | the terrain cell width | — |
| `+0x34`, `+0x38`, `+0x3C` | the last snapped cursor position | −1.0, −1.0, −1.0 |
| `+0x40` | the `local_tile_outline` effect | null |
| `+0x44` | a local grid helper (local-grid-helper.md) | constructed |
| `+0x48` | the network tool (`cISC4NetworkTool`), held | null |
| `+0x4C` | its UI front end (`cSC4NetworkToolUI`, network-tool-ui.md), held | null |
| `+0x50` | the network type | the argument |
| `+0x54` | byte: the left button is down | 0 |

### 1.2 Init (`0x00662160`)

Already initialised: return true. Otherwise:

1. `t := NetworkManager().GetNetworkTool(+0x50, true)` (a private tool, network-manager.md §7.1); hold it in `+0x48`
   (AddRef the new, Release the old, nothing when equal). `t.Init()` (slot 3).
2. Release and clear `+0x4C`; `t` slot 6 creates the UI front end into `+0x4C` (the factory `0x00623B20`,
   network-tool-ui.md §2).
3. Control ID `+0x0C` := UI slot 4 (`ViewInputControlId`); cursor ID `+0x10` := UI slot 5 (`CursorId`).
4. `r := cSC4ViewInputControl::Init()`. When `r`: UI slot 16 `AttachToView(+0x1C)`, then UI slot 7 `EnterUI`.
5. Subscribe the message target `+0x28` to `0x09C8B8EA` (message server `0x00B43CCC`, slot 5).
6. With the terrain: `+0x30` := its cell width; `+0x2C` := its view (AddRef when non-null).
7. Initialise the grid helper; when `+0x40` is null, create `local_tile_outline` into it (effects manager
   `0x00B43D1C` slot 7).
8. Initialised := 1; return `r`.

### 1.3 Shutdown (`0x00661DC0`) and destruction

Not initialised: return true. Otherwise, in order: `ClearCursorText(0xE9C87F80)`; `ClearCostIndicator(0)`; shut the
grid helper; release and clear `+0x2C`; the outline: `Stop(0)` when started, release, clear; unsubscribe `+0x28` from
`0x09C8B8EA` (slot 6); UI slot 8 `LeaveUI`; UI slot 16 `AttachToView(null)`; the tool's `Shutdown` (slot 4), release,
clear `+0x48`; UI slot 3 `Shutdown`, release, clear `+0x4C`; return `cSC4ViewInputControl::Shutdown()`.

The deleting destructor (`0x00662100`) releases `+0x4C` and `+0x48` (no Shutdown calls), destroys the grid helper,
releases `+0x40`, runs the base destructor and frees with `GlobalHeapFree` when flag bit 0 is set. It does not release
`+0x2C`.

### 1.4 Keys and buttons

- **OnKeyDown(vkey, modifiers)** (`0x00661E90`): only when the control is on top (`IsOnTop`) and `vkey` is `0x1B`
  (Escape): when the UI exists and is active (slot 13), `CancelInput` (slot 12); otherwise `EndInput` (own slot 27).
  Either way return true. Any other key, or not on top: false.
- **OnMouseDownL(x, z, m)** (`0x00661EE0`): `KillKeyboardScrolling`; `SetCapture`; `+0x54` := 1; stop the outline
  when it is started; hide the grid effect (`ShowEffect(false)`); `r := UI.OnMouseDownL(x, z, m)` (slot 9);
  `SetCursor(UI.CursorId())` (own slot 8); return `r`.
- **OnMouseUpL(x, z, m)** (`0x00661F50`): when `+0x54` is clear return false. Otherwise `ReleaseCapture`; `+0x54` :=
  0; start the outline (`Start(0)`) when it exists and is not started; `r := UI.OnMouseUpL(x, z, m)` (slot 10);
  `SetCursor(UI.CursorId())`; return `r`.
- **OnMouseExit** (`0x00661FC0`): `ReleaseCapture`; stop the outline when started; `ShowEffect(false)`;
  `ClearCursorText(0xE9C87F80)`; `ClearCostIndicator(0)`; return true. `+0x54` is not cleared.

### 1.5 OnMouseMove(x, z, m) (`0x00662290`)

1. `P :=` the last position (`+0x34..+0x3C`); `hit := PickTerrain(x, z, &P, 0)`. On a hit, snap P to the cell centre
   with `w := +0x30`.
2. `changed :=` any component of P differs from the stored one by at least `1e-4` (`0x00AAAFF4`) or is NaN. Store P.
3. No hit, or `AmCapturing()` (own slot 25): stop the outline when started; `ShowEffect(false)`. Go to step 6.
4. Not changed: go to step 6.
5. Changed: `SetPosition(&P)` on the grid helper; `ShowEffect(terrain view slot 20 is false)`. When the outline
   exists:
   1. `(n, ·) := UI.PenWidth()` (UI slot 6: two output words, the type information's `+0x1C` and `+0x20`; only the
      first is used).
   2. When `n > 1`: `d := (n − 1)·w·0.5` (`0x00A84D2C`); `P.x −= d`, `P.z −= d` (local copy only).
   3. Descriptor: identity matrix, translation `(P.x, P.y, P.z)`, scale `s := n·w`; flag bytes 2 and (`s == 1.0` ? 1 :
      3). `SetTransform`; `Start(0)` when not started.
6. `r := UI.OnMouseMove(x, z, m)` (slot 11), whatever happened above.
7. When `hit` and `+0x54`: `base := UI.DirectionalCursorBase()` (slot 14). If modifier bit `0x10` is clear (the
   button is no longer held): cancel the UI input when it is active (slots 13, 12) and return **true**. Else, when
   `base ≠ 0` and `d := UI.LastDragDirection()` (slot 15) is ≥ 0: `rot :=` the camera control's rotation (renderer
   `= view slot 4`, its slot 16, `+0x10C`, camera.md); `SetCursor(((d − 2·rot) & 7) + base)`.
8. Return `r`.

### 1.6 DoMessage (`0x00662010`, on `+0x28`)

Only `0x09C8B8EA` (the view-input-control change; void2, message slot 15, is the new control) is handled; it returns
true, any other type false.

- New control = this control: when `+0x54` is set and the primary mouse button is still down
  (`GetAsyncKeyState(GetSystemMetrics(SM_SWAPBUTTON) ? VK_RBUTTON : VK_LBUTTON) < 0`), `SetCapture` again.
- Another control: unless its ID (slot 5) is `0x48E2A1F8` (the right-drag scroller), cancel the UI input (slot 12,
  when the UI exists), call own slot 22 (`OnMouseExit`) and clear `+0x54`.

---

## 2. The intersection tool (`cSC4ViewInputControlNetworkIntxTool`) *[compat]*

### 2.1 Identity and state

Vtable `0x00AAAF20`: overridden slots 3 `Init` `0x00660CA0`, 4 `Shutdown` `0x00660D30`, 14 `OnKeyDown`
`0x00660E80`, 16 `OnMouseDownL` `0x00661010`, 18 `OnMouseUpL` `0x00660C30`, 20 `OnMouseMove` `0x00661260`, 22
`OnMouseExit` `0x00660DE0`, 26 the deleting destructor `0x006611E0` (body `0x00661200`). The 3D view creates it for
the commands `0x6A470000…0x6A47FFFF`: `0x90` bytes, the command id's low 16 bits as the starting rule id
(`0x007EA373…0x007EA3A5`).

| Offset | Meaning | Constructor (`0x00661130`) |
|---|---|---|
| `+0x28` | the terrain view, held | null |
| `+0x2C` | the terrain cell width | — |
| `+0x30`, `+0x34`, `+0x38` | the last snapped cursor position | −1.0 ×3 |
| `+0x3C` | the `local_tile_outline` effect | null |
| `+0x40` | the intersection preview effect | null |
| `+0x44` | a local grid helper | constructed |
| `+0x48` | the best rule found by the last search (a rule pointer, not held) | 0 |
| `+0x4C` | the current rule id | the argument |
| `+0x50`, `+0x54` | the placement cell (x, z) | — |
| `+0x58`, `+0x5C` | the placement cost (int64, low, high) | — |
| `+0x60` | a string object: the cursor text (buffer `+0x64…+0x6C`) | empty |
| `+0x74` | a string object: the name of the preview effect held (buffer `+0x78…+0x80`) | empty |
| `+0x88` | byte **placeable** | 0 |
| `+0x89` | byte, cleared before each dry placement; nothing here reads it | — |

The destructor body destroys both strings and the grid helper, releases `+0x40` and `+0x3C` and runs the base
destructor; it releases neither `+0x28` nor any rule.

Rules are looked up in the global rule map (network-rules.md §10.1) by id, through `GetIntersectionRule(id)`
(`0x00625380`) or the same bucket walk inline: no rule → the id is kept where a "next id" is wanted. Rule fields
used: `+0x04` the model list (16-byte entries `{id, rot byte, flip byte, x, z}`), `+0x7C` `PlaceQueryID`, `+0x80` the
`PreviewEffect` string, `+0x94`/`+0x98` the handle offset, `+0xEC`/`+0xF0` the rotation ring (previous, next),
`+0xF4`/`+0xF8` the type ring (previous, next).

### 2.2 Init, Shutdown, exit

- **Init** (`0x00660CA0`): initialised → true. Cursor ID `+0x10` := `0x88E70B76`; `r := cSC4ViewInputControl::Init()`;
  with the terrain, `+0x2C` := cell width, `+0x28` := its view (AddRef); initialise the grid helper; create
  `local_tile_outline` into `+0x3C` when null; initialised := 1; return `r`.
- **Shutdown** (`0x00660D30`): not initialised → true. Otherwise it **clears the initialised flag first**, then:
  `ClearCursorText(0xEA282CA4)`; `ClearCostIndicator(0)`; shut the grid helper; release and clear `+0x28`; the
  outline: stop when started, release, clear; the preview: `Stop(0)`, release, clear, and empty the name `+0x74`;
  return `cSC4ViewInputControl::Shutdown()`, which now finds the flag clear and does nothing *[quirk]*: the view,
  the window manager and the cursor stay held.
- **OnMouseExit** (`0x00660DE0`): `ReleaseCapture`; stop the outline when started; remove the preview (as in
  Shutdown); `ShowEffect(false)`; `ClearCursorText(0xEA282CA4)`; `ClearCostIndicator(0)`; return true.

### 2.3 OnKeyDown(vkey, modifiers) (`0x00660E80`)

Not on top: false. The keys (jump tables `0x00660FD4`/`0x00660FE8`):

| Key | Effect on the current id `+0x4C` |
|---|---|
| Tab `0x09`, modifier bit 0 set | the rule's type-ring previous (`+0xF4`) |
| Tab, bit 0 clear | the rule's type-ring next (`+0xF8`) |
| End `0x23` | the rule's rotation-ring next (`+0xF0`) |
| Home `0x24` | the rule's rotation-ring previous (`+0xEC`) |
| Escape `0x1B` | none: `EndInput` (slot 27), return true |
| other | none: return false |

After a ring step: read the cursor position in the view window (`view.AsIGZWin()`, its window manager — window slot 6
— slot 34 `GetCursorPosition(window, &x, &y)`), set `+0x30` := `1e10` (`0x501502F9`) so the move counts as a change,
call own slot 20 `OnMouseMove(x, y, 0)` and return true.

### 2.4 OnMouseMove(x, z, m) (`0x00661260`)

The handler **always returns false**.

1. Pick, snap and compare exactly as §1.5 steps 1–2 (`w := +0x2C`; threshold `1e-4` at `0x00AAAF18`); store P.
2. No hit, or `AmCapturing()`: remove the preview (stop, release, clear, empty `+0x74`); stop the outline when it is
   started (the outline pointer is **not** null-tested here *[quirk]*); `ShowEffect(false)`. Go to step 12.
3. Not changed: go to step 12.
4. `SetPosition(&P)` on the grid helper; `ShowEffect(terrain view slot 20 is false)`. When the outline exists: identity
   matrix, translation P, scale `w`, flag bytes 2 and (`w == 1.0` ? 1 : 3); `SetTransform`; `Start(0)` when not
   started.
5. **The cursor cell**: `cx := floor(P.x / 16)`, `cz := floor(P.z / 16)` (the constant `0.0625` at `0x00AA66E0`, the
   `1.5·2²³` rounding with the correction of effects-owners.md's notation, not the stored cell width). `+0x50` := cx,
   `+0x54` := cz. Empty failure text S; placeable `+0x88` := 0; best `+0x48` := null.
6. **The search.** `(W, H) :=` the city's cell counts (city slots 93, 94). `T := GetNetworkTool(2, false)` (the shared
   highway tool); `T.Init()`. `start :=` the current id. For each offset `(dx, dz)` of the table `0x00AAAFCC`, in
   order `(0,0)`, `(1,0)`, `(0,−1)`, `(−1,0)`, `(0,1)`, walk the rotation ring from the current id:
   1. `R :=` the rule of the current id. When `R` exists: `px := cx − R.+0x94 + dx`, `pz := cz − R.+0x98 + dz`. When
      `R.IsWithinBounds(px, pz, W, H)` (network-rules.md §10.5):
      1. `T.Reset()` (slot 5); `R.DoAutoComplete(px, pz, T)`; `+0x89` := 0;
         `ok := T.InsertIsolatedHighwayIntersection(px, pz, id, false)` (a dry run, network-rules.md §10.7).
      2. Not ok: when `R` is the best rule and the tool's failure code `+0x20C` is non-zero, S := its reason text
         (`T` slot 52, `GetFailureReasonString(code, &S)`). Ok: the cost `+0x58` := `T.GetCostOfSolution()`, `+0x5C`
         := 0.
      3. When there is no best rule yet, or ok: best := R. (So the first rule within bounds becomes the best, and its
         failure text is only taken on a later attempt with the same rule, at a later offset *[quirk]*.)
      4. `T.Reset()`. When ok: placeable := 1, `+0x50` := px, `+0x54` := pz; end the search.
   2. The current id := R's rotation-ring next (unchanged when there is no rule). Stop this offset's walk when it is
      back at `start`.

   The current id is left on the rule that was placed, so the search may turn the tool to the first rotation that
   fits. After the search: `T.Shutdown()` (slot 4).
7. **Text, cost, cursor.** Placeable: the text := the string `(group 0x2A592FD1, instance R.PlaceQueryID)`
   (`SC4LoadString`), or `## Intersection placement string missing ##` when the load fails; `SetCursor(0x417EE36B)`;
   `SetCursorText(0xEA282CA4, 0, 0, &text, 0)`; `SetCostIndicator(+0x58)`. Not placeable: `ClearCostIndicator(0)`;
   when S is not empty, text := S and `SetCursorText(0xEA282CA4, …)`, else `ClearCursorText(0xEA282CA4)`;
   `SetCursor(0x88E70B76)`. (An 8-byte buffer is allocated and freed around the placeable branch; nothing uses it.)
8. **Funds.** When the city's funds (city slot 52, the budget; its slot 6, int64) are below the cost `+0x58/+0x5C`
   (signed 64-bit compare): placeable := 0. The text and cost of step 7 stay.
9. **The preview's rule.** Not placeable: R := best; none → step 11. Then `+0x50 −= R.+0x94`, `+0x54 −= R.+0x98`.
   This also happens when step 8 cleared placeable, so an unaffordable intersection's preview is shifted by its
   handle offset a second time *[quirk]*. Placeable: R is the rule that was placed.
10. **The preview.** When R has a model and a non-empty `PreviewEffect`: with M its first model,
    `wx := (+0x50 + 0.5)·16 + M.x`, `wz := (+0x54 + 0.5)·16 + M.z`.
    1. `wx < 0`, `wz < 0`, `wx > 16·cellCountX` or `wz > 16·cellCountZ` (terrain slots 7, 8): remove the preview; step
       11.
    2. `y := GetAltitude(wx, wz) + 0.5`. `name := PreviewEffect`, plus `_red` when not placeable. When `name` differs
       from `+0x74`: remove the preview; create `name` into `+0x40` (effects manager slot 7); on success `+0x74` :=
       name and `Start(0)`.
    3. When a preview is held: `θ := M.rot·(−π/2)` (`0x00AA88B4`), `c := cos θ`, `s := sin θ`. Matrix rows `(c, 0, s)`,
       `(0, 1, 0)`, `(−s, 0, c)`; with M's flip byte set, the first row is negated `(−c, 0, −s)`. (The original forms
       each entry as `c − 0·s`, `s + 0·c` and so on with a stored zero; the values are c and s.) Translation
       `(wx, y, wz)`, scale 1.0, flag bytes 2 and 5. Send it with the preview's slot 7 (the secondary transform).
    R with no model or an empty `PreviewEffect`: the preview is left as it was *[quirk]*.
11. Destroy S.
12. Flush queued keyboard input: the graphics system (`0x00B43C9C`) slot 14 gives the canvas, its slot 18
    (gz-graphics.md). Return false.

### 2.5 OnMouseDownL and OnMouseUpL

Both return false (AL 0, pop 12).

- **OnMouseDownL** (`0x00661010`): `SetCapture`; stop the outline when started. Not placeable: play sound
  `0x495A6D86` (sound system `0x00B43CB8`, AddRef, slot 11 `(1, id, 0, 0, 0)`, Release). Placeable:
  `T := GetNetworkTool(2, false)`; `T.Init()`; `T.Reset()`; `rule(+0x4C).DoAutoComplete(+0x50, +0x54, T)`;
  `T.InsertIsolatedHighwayIntersection(+0x50, +0x54, +0x4C, true)` (the result is not checked); `T.Reset()`;
  `T.Shutdown()`; spend the cost (the budget's slot 8 with `(+0x58, +0x5C)`), whatever the insert returned *[quirk]*;
  play sound `0xEA5EC59F`.
- **OnMouseUpL** (`0x00660C30`): `ReleaseCapture`; start the outline when it exists and is not started; when not
  placeable, play sound `0xAA62C9D5`; placeable := 0.

---

## 3. The toll booth tool (`cSC4ViewInputControlTollBooth`) *[compat]*

Vtable `0x00A92AC0`; it overrides only `Init` (`0x004D9E50`), `Shutdown` (`0x004D9EA0`) and `OnMouseMove`
(`0x004DA1B0`); the destructor is the folded `0x004D7AA0` (body `0x004DA180`). It has **no click handler**: it only
shows where a toll booth would go. It is installed by the `toll` cheat (view3d-window.md §3.1; `0x38` bytes, constructor
`0x004D9E80`: `+0x28`, `+0x2C`, `+0x34` := 0).

| Offset | Meaning |
|---|---|
| `+0x28` | the preview prop (`cISC4PropOccupant`), held |
| `+0x2C` | the footprint effect, held |
| `+0x30` | the last state (Init sets 1) |
| `+0x34` | the last lot configuration id |

- **Init**: when not initialised: control ID `+0x0C` := `0x8BFC65A7`, cursor `+0x10` := `0x88E70B76`, the base `Init`,
  `+0x30` := 1. Returns true either way.
- **Shutdown**: when initialised: the footprint effect `Stop(0)`, release, clear; the base `Shutdown`. Returns true.
  The preview prop is not removed from the prop manager; the destructor only releases `+0x2C` and `+0x28`
  *[quirk]*.

### 3.1 PickLotAndState(x, z, &state, &config, &orientation, &pos) (`0x004D9EE0`, static, pop 24)

1. `state`, `config`, `orientation` := 0. `pos.x, pos.z :=` the cell centre (city slot 97
   `CellCenterToPosition(x, z, &pos.x, &pos.z)`); `pos.y` is not written.
2. `o :=` the network occupant at the cell (the traffic network `0x00B43D54`, slot 8 `(x, z, 0x144D, true)`); none:
   return.
3. `t :=` the first of 0, 2, 3, 6, 10, 12 for which `o` slot 22 `(t)` is true; none: return.
4. The occupant's flags (slot 12) masked by `0x1F4F` must hold no bit other than `1 << t`; else return.
5. `sides := o` slot 38 `(t)`: four bytes, side 0 in the low byte (sides 0–3 are −x, −z, +x, +z, as in
   city-tools-input.md §3.13). A side whose low two bits are 1 or 3 returns at once; 2 is connected; bit 2 is the
   side's alignment marker. Exactly two sides must be connected, else return.
6. By type (jump tables `0x004DA160`/`0x004DA16C`):

| Types | Connected | state | orientation | pos change |
|---|---|---|---|---|
| 0, 3, 10 | 0 and 2 | 1 | 2 | none |
| 0, 3, 10 | 1 and 3 | 1 | 1 | none |
| 2, 6, 12 | 0 and 2 | 2 | 2 | `pos.z −= 8` when side 1's marker is set, else `+= 8` |
| 2, 6, 12 | 1 and 3 | 2 | 1 | `pos.x −= 8` when side 0's marker is set, else `+= 8` |

   Any other pair: return with all three outputs 0. On success `config :=` the table `0x00A92A88` by type: road
   `0xEBF9A440`, highway `0x8BF9B356`, street `0x2BF9B3A8`, avenue `0xABF9A45C`, type 10 `0x4BF9B3E1`, type 12
   `0x2BF9B222` (8.0 at `0x00A8E178`).

### 3.2 OnMouseMove(x, z, m) (`0x004DA1B0`)

Always returns true.

1. Pick the terrain through the global 3D view `0x00B43DC4` (slot 36), the position preset to the zero vector
   `0x00B2192C`; a miss returns true.
2. The cell: city slot 95 `PositionToCell`; P := its centre (city slot 97). `PickLotAndState(cell, &state, &config,
   &orientation, &Q)` with Q preset to zero; `Q.y := GetAltitude(Q.x, Q.z)`.
3. When `state ≠ +0x30`: stop (when started), release and clear the footprint effect; `+0x30` := state; create
   `building_footprint_notok` for state 0, `building_footprint_ok` for 1 or 2 (effects manager slot 7). Because Init
   sets `+0x30` to 1, the first valid cell creates no effect *[quirk]*.
4. With a footprint effect: `Start(0)` when not started; read its secondary transform (slot 15) into an identity
   descriptor, set the translation to Q, set bit 0 of the second flag byte and add 1 to the first, `SetTransform`.
5. When `config ≠ +0x34`: `+0x34` := config; when a preview prop is held, remove it (prop manager `0x00B43D10` slot
   29 `RemoveProp(prop.AsOccupant())`), release, clear. Then, when config ≠ 0 and the lot configuration exists (lot
   configuration manager `0x00B43D18` slot 8):
   1. `(cw, cd) :=` its size (slot 7); B := its primary building entry (slot 24): `+0x04…+0x0C` local position, first
      id at `*(+0x28)`. `local := (B.x − 8·cw, B.y, B.z − 8·cd)` (from `cw·16 >> 1` as an unsigned integer).
   2. `key :=` the building key of the first id (building development `0x00B43D68`, slot 14). On success create a
      generic prop from it (prop manager slot 16 `CreateGenericProp(key, &+0x28)`); on success: occupant slot 29
      `SetFlag(0x10)`, the booleans `0x029244DB` (preview occupant) and `0x29F013E6` true on its property holder
      (slot 6, `SetPropertyValue`), occupant slot 9 `SetPosition(&local)`.
6. With a preview prop: occupant slot 9 `SetPosition(&Q)` (so the local position of 5.1 never survives *[quirk]*);
   prop slot 10 `SetOrientation(orientation)`.

---

## 4. The network preview (`cSC4NetworkPreview`) *[compat]*

The cell quads a network tool draws while drafting (the tool's highlights; the power-line tool's slot 82 is one user,
power-line-tool.md). Callers are in `cSC4NetworkTool` (`0x006247D9`…`0x00628883`, `0x0064F34E`).

### 4.1 State

| Offset | Meaning |
|---|---|
| `+0x0C` | an occupant filter, held (`cSC4NetworkTypeOccupantFilter`, vtable `0x00AA42FC`, type mask `0x1F4F`) |
| `+0x10` | the untextured cells: a multimap from key to cell record (header pointer; count `+0x14`) |
| `+0x1C` | the textured cells: a multimap from key to {cell record, texture binding} (count `+0x20`) |
| `+0x28`, `+0x30` | packed render states of the untextured passes 0 and 1 |
| `+0x38`, `+0x40` | packed render states of the textured passes 0 and 1 |
| `+0x48` | a 12-byte texture-combiner record |
| `+0x54` | byte: initialised |

The **key** of a cell `(x, z)` is `z·65536 + x`. Insertion is `insert_equal` (after equal keys), so a cell may hold
several quads; drawing follows key order, then insertion order. A **cell record** (`0x68` bytes) holds four vertices of
24 bytes (`x, y, z` floats, a packed colour dword at `+0x0C`, `u, v` floats), the four corner indices (bytes
`+0x60…+0x63`) and the height offset (float `+0x64`). A textured node also holds a texture binding (`+0x68` of the
value, AddRef'd).

### 4.2 Init (`0x0061FD10`)

Once (`+0x54`). When the renderer `0x00B43DD0` exists: its slot 10 (result ignored); `b := !kPreferOpaque` (render
property boolean 33, render-properties.md); pack (draw-context.md §5.1, argument order):

| State | Arguments |
|---|---|
| `+0x28` | `(1,1,1,0, b, 0, 1, 3, 4,5, −1, −1.0, 1, −1)` |
| `+0x38` | `(1,1,1,0, b, 1, 1, 3, 4,5, −1, −1.0, 1, −1)` |
| `+0x30` | `(1,1,1,0, b, 0, 1, 4, 4,5, −1, −1.0, 1, −1)` |
| `+0x40` | `(1,1,1,0, b, 1, 1, 4, 4,5, −1, −1.0, 1, −1)` |

(pass 0 depth function 3, less-or-equal; pass 1 depth function 4, greater). The combiner bytes `+0x48…+0x4A` := 2,
0, 3 and `+0x4C…+0x4E` := 1, 0, 3. A new filter (`0x10` bytes, mask `0x1F4F` at `+0x0C`) replaces `+0x0C` (AddRef
the new, Release the old). Then, renderer or not, initialised := 1. Returns true.

### 4.3 Building a record

**SetVertexPositions(rec, &cell, colour, minHeights, uvFlags, mirrorU, heightOffset, flat)** (`0x0061FE60`, pop 32):

1. `d :=` the terrain's `IsTriangulationFlipped(cell)` (slot 70) as 0 or 1; the corner order is the row d of
   `0x00AA82D0`: `(0,1,2,3)` or `(1,2,3,0)`. Corner k has offsets `dx = (0,0,1,1)[k]`, `dz = (0,1,1,0)[k]`
   (`0x00AA82AC`, `0x00AA82BC`).
2. For each vertex i = 0…3 with corner `k := order[i]`: corner byte `+0x60+i` := k; `x := (cell.x + dx[k])·16`,
   `z := (cell.z + dz[k])·16`; colour := `colour`; `(u, v) := (dx[k], dz[k])`, then `u ^= 1` when `mirrorU`, then
   `(u, v) := (v, u ^ 1)` when `uvFlags` bit 0, then `u ^= 1, v ^= 1` when bit 1.
3. `+0x64` := heightOffset; `SetVertexHeights(rec, &cell, minHeights, flat)`.

**SetVertexHeights(rec, &cell, minHeights, flat)** (`0x0061FBF0`, static, pop 16): `water :=` terrain slot 83. For each
vertex with corner k at `(px, pz) = (cell.x + dx[k], cell.z + dz[k])`: `y := 0`; when not `flat`:
`y := GetAltitudeAtVertex(px, pz)` and, when `water`, `y += max(GetWaterDepthAtVertex(px, pz), 0)` (computed as
`(|w| + w)·0.5`). When `minHeights` is given and `minHeights[k] > y`: `y := minHeights[k]`. Finally `y += +0x64`.

### 4.4 Adding, adjusting, erasing

- **AddUntexturedSelection(&cell, colour, minHeights, heightOffset, flat)** (`0x00620820`, pop 20): a record by
  `SetVertexPositions(…, uvFlags 0, mirrorU 0, …)` inserted into `+0x10`.
- **AddTexturedSelection(&cell, colour, minHeights, textureId, uvFlags, mirrorU, heightOffset, flat)** (`0x00620BC0`,
  pop 32): a record as above; the binding for `textureId` from the texture service `0x00B43CD4` (slot 6
  `(id, &binding)`, network-occupants.md). Found: insert {record, binding} into `+0x1C` (AddRef). Not found:
  `AddUntexturedSelection(cell, colour, minHeights, heightOffset, flat)` instead. Temporary references are released.
- **AdjustCellHeights(&cell, minHeights, flat)** (`0x00620930`): for every record of the cell's key in both maps,
  `SetVertexHeights(record, cell, minHeights, flat)` (with the record's own corners and height offset).
- **Erase(&cell)** (`0x00620CF0`): erase the cell's equal range from both maps (`0x006202E0`, `0x006206A0`; the
  textured erase releases each binding; a range covering the whole map clears it at once). The node helpers
  `0x0061FCD0` (recursive free), `0x006204B0`, `0x006205C0`, `0x00620A90` (textured node copy and insert) are the
  ordinary multimap operations.

### 4.5 Drawing: Update(dc) (`0x00620500`, the draw-callback slot) and DrawCells(dc) (`0x00620160`)

`Update`: save the draw context's depth offset (`+0xBC`); `SetDepthOffset(4)`; reset the model transform and set an
identity one (flag bytes 0, identity matrix, translation 0, scale 1); `DrawCells`; restore the depth offset; return
true.

`DrawCells`, for pass p = 0, 1: p = 0 `ResetTransparency`, p = 1 `SetTransparency(0.2)` (`0x3E4CCCCD`).

1. `SetRenderState(+0x28 + 8p)`; for each untextured record in map order: `SetVertexBuffer(10, 4, record)`,
   `DrawPrimsIndexed(0, 6, {0,1,2, 0,2,3})` (u16 indices at `0x00AA82F0`).
2. `SetRenderState(+0x38 + 8p)`; `SetTexWrapModes(2, 2, 0)`; `SetTexCombiner(+0x48, 0)`; for each textured record:
   `SetTexture(binding's texture, 0)`, the same buffer and draw.

Then `ResetTransparency`. So every quad is drawn once normally and once at 20% where it is hidden (depth greater).

---

## 5. `cISC4NetworkTool::tPlacementParams::operator=` (`0x00626860`) *[compat]*

A member-wise copy of the `0xAC`-byte record of a network type's placement parameters; returns `this`. Words
`+0x00…+0x20`, bytes `+0x24`, `+0x25`, a `vector<uint32>` `+0x28`, words `+0x34…+0x40`, byte `+0x44`, words
`+0x48…+0x50`, `vector<uint32>` `+0x54` and `+0x60`, `vector<float>` `+0x6C`, words `+0x78…+0x9C`, `vector<uint32>`
`+0xA0` (each vector by value). The network tool's `Init` copies its type information's record (`+0x358`, `+0x50`)
into `+0x290` (`0x00626AFC`); `GetPlacementParams(out)` (`0x0062CAD0`) copies `+0x290` out.

---

## 6. Correction to network-tool-ui.md §3

The UI front end's slots 4, 5, 6 and 14 forward to network tool slots **42, 43, 45 and 44** (`0x006452E0` jumps
through `+0xA8`, `0x006452F0` `+0xAC`, `0x00645300` `+0xB4`, `0x006457B0` `+0xB0`), not 42/43/44/33. The base tool
bodies (`0x0062C910`, `0x0062C930`, `0x0062C940`, `0x0062C950`) read the type information `+0x358`: slot 42 its
`+0x08`, 43 `+0x14`, 44 `+0x18`, and slot 45 writes `+0x1C` and `+0x20` to two output words (pop 8). So `PenWidth`
(UI slot 6) takes two output pointers.

## 7. Quirks

1. The intersection tool's Shutdown clears its flag before calling the base Shutdown, which then releases nothing
   (§2.2).
2. The intersection search takes the failure text only from a later attempt of the best rule (§2.4 step 6).
3. An unaffordable intersection's preview is shifted by the handle offset twice (§2.4 step 9).
4. The intersection tool's mouse move returns false and, on a miss, does not null-test the outline (§2.4).
5. A placed intersection is paid for even when the real insert fails (§2.5).
6. The toll booth tool never removes its preview prop on Shutdown, creates no effect for the first valid cell, and
   overwrites the prop's local position at once (§3).

## 8. Conformance tests

1. Network tool control, pen width 3, cell width 16, cursor at (40, h, 70): P snaps to (40, h, 72); the outline is
   placed at (24, h, 56) with scale 48 and flag bytes 2, 3.
2. Same with pen width 1: translation (40, h, 72), scale 16, flags 2, 3; with a cell width of 1.0 and pen width 1 the
   second flag byte is 1.
3. Escape while the UI is active cancels it and the control stays; a second Escape ends the input.
4. Intersection tool on rule A whose rotation ring is A → B → A: with A out of bounds at every offset and B failing at
   offset (0, 0) but fitting at (1, 0), the current id becomes B, `+0x50 = cx − B.hx + 1`, placeable, and the cursor
   is `0x417EE36B`; the failure text of B's first attempt is not shown.
5. Intersection tool with funds below the cost: the placement text and cost still show, the cursor is the placement
   one, but the preview uses `<PreviewEffect>_red` and a click plays `0x495A6D86` and spends nothing.
6. Toll booth over an east-west road (sides 0 and 2): state 1, orientation 2, configuration `0xEBF9A440`, the prop at
   the cell centre; over a highway with side 1's marker set: Q.z is 8 less.
7. Preview: a cell at (3, 5) gets key `0x00050003`; with a flipped triangulation its first vertex is corner 1 at
   (48, ·, 96) with uv (0, 1); with `uvFlags` 1 that uv becomes (1, 1).
8. `tPlacementParams` copy then compare: every word, byte and vector element equal; the source is unchanged.
