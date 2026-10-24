# `cSC4GraphsViewWinProc`: the city graphs panel (behaviour spec)

Scope: the whole `cSC4GraphsViewWinProc` and `cSC4LineGraph` owners as tools/coverage.py assigns them (35 + 2
functions, 11,633 + 59 bytes on `5c0ff782`; inventory in §9). Four of the 35 belong in fact to the line-graph widget
(§7). The graph widgets
(`cGZGraph`, `cGZLineGraph`, `cGZBarGraph`, `cGZPieGraph`) are not specified here; the panel drives them through
their interfaces, and their slots are named by number only (§6). The data source is the history warehouse
(history.md); the panel is created by the city view (view3d-window.md, `+0x2F8`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was disassembled; the 18 shipped graph definitions were read from
`SimCity_1.dat` with `tools/dbpf.py`. Written 2026-10-23 (re-graphs7).

Conventions:
- `W` is the panel object.
- `G` is the graph widget's graph interface (`widget + 0xD8`).
- `L` and `B` are the widget's line and bar interfaces (QI `0x12CF2351` and `0x22CF2351`).
- `H` is the history warehouse (city `0x00B43CEC` slot 32), and `H.Get(key, date, &v)` is its slot 8.
- `sim` is the simulator `0x00B43D5C`: slot 5 `GetSimStartDate(&date)`, slot 8 `GetSimDateNumber()`.
- Colours made with `RZ::GS` slot 34 `(r, g, b)` are written `rgb(r, g, b)`.
- Strings `text(i)` are LTEXT group `0x6A231EAA`.

## 1. Identity and lifetime *[compat]*

| Offset | vtable | Interface | Slots |
|---|---|---|---|
| `+0x00` | `0x00AB4FF4` | message target | 0 QI (`0x007723F0`, shared), 1 AddRef, 2 Release, 3 `DoMessage`, 4 the deleting destructor (`0x0076D290`) |
| `+0x04` | `0x00AB4FE0` | `cIGZWinProc` | 0 QI, 1 AddRef, 2 Release, 3 `DoWinProcMessage`, 4 `DoWinMsg` (shared) |

- **Constructor** (`0x0076CFE0`, RET 4, the argument is the main window):
  - `+0x14` := 1 and `+0x29` := 1;
  - `+0x18` := the main window, AddRef'd;
  - the visibility map `+0x38` gets an empty header (0x18 bytes);
  - all other fields 0.
- **Destructor** (`0x0076D080`):
  1. `Shutdown`, then release the main window.
  2. Clear and free the visibility map; destroy the series records and free their vector.
  3. Release `+0x24`, `+0x20`, `+0x1C` and `+0x18` (the last again: null after the first release).
- **`Init`** (`0x0076CA60`):
  1. The first time only:
     - `p` = the main window's parent (slot 11). The collapsed panel `+0x1C` := `p` slot 34 `(0x8A8B5B72)` and the
       expanded panel `+0x20` := `p` slot 34 `(0x8A8B5B71)`, counted.
     - `SetTimeScale(0)`; hide both panels (slot 70).
     - The main window and both panels get `W+4` as window procedure (their `cIGZWinGen`, IID `0x5386D516`, slot 7)
       and window flag `0x800` cleared (`SetFlag(0x800, 0)`).
     - Subscribe `W` to message `0x66956816` on `0x00B43CCC`; set `+0x0C`.
  2. Every call:
     - the buttons `0x0A8B7E01` (in `+0x1C`), `0x0A8B7E00` (in `+0x20`) and `0xCA8B82F9` (in both) get the main
       window as notification target;
     - when the main window has child `0xAA9244B6`, its slot 32 walks its `cIGZWin` children with `0x0076B5D0`,
       which makes the main window each child's notification target.
  3. True.
- **`Shutdown`** (`0x0076B750`), when initialised:
  1. Clear `+0x0C`; unsubscribe `0x66956816`.
  2. Remove the graph from `+0x20` (slot 16), release it.
  3. Clear the window procedure of the main window and both panels (slot 7 with 0); release both panels.
  4. True.

Layout:

| Offset | Field |
|---|---|
| `+0x0C` | byte: initialised |
| `+0x10` | the current graph kind: 1 line, 2 bar, 3 pie |
| `+0x14` | the current graph id |
| `+0x18`, `+0x1C`, `+0x20` | the main window, the collapsed panel, the expanded panel |
| `+0x24` | the graph widget (counted) |
| `+0x28` | byte: shown |
| `+0x29` | byte: expanded |
| `+0x2C`…`+0x34` | vector of series records (0x30 bytes each) |
| `+0x38`, `+0x3C` | map key → byte: the remembered visibility of a series key, and its count |
| `+0x44` | the number of series |
| `+0x48` | the number of bar groups |
| `+0x4C` | the time-scale index (§2.2) |
| `+0x50` | the axis maximum, `−FLT_MAX` (`0xFF7FFFFF`) when unset |
| `+0x54` | the axis minimum, `FLT_MAX` (`0x7F7FFFFF`) when unset |
| `+0x58` | the axis step, 0 when unset |

A **series record** (0x30 bytes):

| Offset | Field |
|---|---|
| `+0x00` | the history key |
| `+0x04` | the second key, for the operation |
| `+0x08` | the bar group |
| `+0x0C` | the operation (§5.2) |
| `+0x10`…`+0x12` | the line colour (r, g, b) |
| `+0x14` | float: the scale |
| `+0x18` | the label (`cRZString`) |
| `+0x2C` | byte: visible |

## 2. Data *[compat]*

### 2.1 Graph definitions

A graph id `g` names the exemplar (`0x6534284A`, `0xCA4AD545`, `g`):

| Property | Meaning |
|---|---|
| `0xAA4C0D1B` | kind: 1 line, 2 bar, 3 pie |
| `0x8A416A99` | the title (`SC4LoadExemplarString`); if absent, the exemplar name `0x20` |
| `0x6A4AEE40` | the series keys (history keys) |
| `0x6A4AEE5F` | the second keys (line) |
| `0x6A4AEE68` | the operations |
| `0x6A4AEEDC` | the label string ids |
| `0x6A4AEEDF` | the line colours, 3 bytes per series |
| `0x6A4AEEE2` | the scales (floats) |
| `0x6A4AEEE3` | the bar group of each series |
| `0x6A4AEEE4`, `0x6A4AEEE5` | the bar fill and edge colours, 3 bytes per group |
| `0xEA4AEDC6`, `0x6A4AEDED`, `0x6A4AEDEF` | the axis minimum, maximum and step |

Shipped (18): line graphs 1–13 (Mayor Rating, Funds, Income & Expense, Average Income, Life Expectancy, Power,
Water, Air Pollution, Water Pollution, Garbage, Traffic, Crimes vs. Arrests, EQ), `0x11` RCI and `0x18` Traffic
Volume. Bar graphs: `0xE` Demands (operation 5 on all 12 series, axis −6000…6000 step 2000), `0x12` Education
Demographic and `0x13` Population Demographic. There is no pie graph.

### 2.2 Time scales (`0x00B0DA50`, 12-byte records)

| Index | Points | Step (days) | Label | Shown span |
|---|---|---|---|---|
| 0 | 12 | 30 | `0x2A5D4165` | 1 year |
| 1 | 60 | 30 | `0xCA5D417F` | 5 years |
| 2 | 120 | 30 | `0xCA5D4184` | 10 years |
| 3 | 100 | 182 | `0x8A5D4189` | 50 years |
| 4 | 100 | 365 | `0x8A5D418E` | 100 years |
| 5 | 100 | 1825 | `0x2A5D4194` | 500 years |

The line graph draws `points + 1` samples (§5.2).

## 3. Showing, hiding and the buttons *[compat]*

- `SetExpanded(on)` (`0x0076B860`): on → hide `+0x1C`, show `+0x20`, `+0x29` := 1; off → the reverse, `+0x29` := 0.
- `SelectOption(g)` (`0x0076B8B0`, RET 4): in the main window, the buttons `0x4000 + old` and `0x4100 + old` (old =
  `+0x14`) get slot 5 `(1, 0)`, then `0x4000 + g` and `0x4100 + g` get slot 5 `(1, 1)` *[inferred: toggle off/on; GW3]*.
- `ShowWindow` (`0x0076E420`):
  1. `ShowGraph(+0x14)`, `SelectOption(+0x14)`; `+0x28` := 1; show the main window.
  2. Show `+0x20` when expanded, else `+0x1C`.
- `HideWindow` (`0x0076E470`):
  - with a main window: `ShowGraph(0)` (the graph is dropped, §4 step 2), hide it, `+0x28` := 0;
  - hide both panels.
- `ShowGraphType(g)` (`0x0076E4B0`, RET 4):
  1. When `g` differs: `SelectOption(g)`, `ShowGraph(g)`, `+0x14` := `g`.
  2. `ShowWindow`; hide `+0x1C`, show `+0x20`, `+0x29` := 1.
- `SetTimeScale(i)` (`0x0076C910`, RET 4): `+0x4C` := `i`. With the expanded panel, its child `0x8A2871D9` (IID
  `0x212CDC1F`) gets slot 4 `(text(label_i))`. With a graph: `RefreshGraph`, then the widget's slot 92 (repaint).
- **`DoMessage(m)`** (`0x0076CC80`, `+0` slot 3): only `0x66956816` (true): with a graph, `RefreshGraph` and repaint.
  Others: false.
- **`DoWinProcMessage(w, m)`** (`0x0076E500`, `+4` slot 3, RET 8):
  1. False unless the main window is visible (slot 71). Only type 3 messages are handled; anything else is false.
  2. `nsSC4TutorialSystem::FilterUICommandMessage(m)` false → true.
  3. By `m[1]` and `m[2]`:

| `m[1]` | `m[2]` | Effect | Returns |
|---|---|---|---|
| `0x887113A3` (slider) | `0xEA4A7CF2` | the slider child's value (slot 4); when it differs from `+0x4C`, `SetTimeScale(value)` | false |
| `0x287259F6` (button) | `0x0A8B7E00` | hide `+0x20`, show `+0x1C`, `+0x29` := 0 (collapse) | false |
| | `0x0A8B7E01` | hide `+0x1C`, show `+0x20`, `+0x29` := 1 (expand) | false |
| | `0xCA8B82F9` | `SetExpanded(!+0x29)` | false |
| | `0x4000`…`0x4FFF` | `g = id & 0xFF`: expand, `SelectOption(g)`, and when `g ≠ +0x14`: `ShowGraph(g)`, `+0x14` := `g` | false |
| | `0x04000000`…`0x0400001F` | a legend checkbox: `RefreshGraph` | false |
| other | | — | false |

The `0x4000` range keeps only the low byte, so ids `0x4100`… (the second button of each graph) select the same
graph.

## 4. `ShowGraph(g)` (`0x0076D3D0`, RET 4) *[compat]*

1. Nothing without the expanded panel.
2. Remove the current widget from the panel (slot 16), release it.
3. Load the definition; without it, return. Read the kind and the title.
4. When the kind differs from `+0x10`: clear the series records and drop the widget again. `+0x10` := kind.
5. Make the widget:
   - 1: a line graph (0x260 bytes; `cSC4LineGraph` constructor `0x0076B5F0`, which sets three vtables on a
     `cGZLineGraph`);
   - 2: `cGZBarGraph` (0x268 bytes);
   - 3: `cGZPieGraph` (0x28C bytes);
   - another kind returns.

   The widget goes in `+0x24`.
6. The line-only controls `0xEA4A7CF2` (the time slider), `0x8A2871D9` (its label) and `0x8A2871D8` are shown for a
   line graph and hidden otherwise, each repainted.
7. The widget takes the area of the panel's child `0x8A4B0AA0` (slot 47 → `SetArea`) and is added to the panel
   (slot 14).
8. **Style**, with `c0 = rgb(0x3F, 0x49, 0x67)`, `c1 = rgb(0xAA, 0xBB, 0xC3)` and the fonts `f1`, `f2` = `RZ::WCM`
   slot 5 of `0xE9C86B5F`, `0xE9C86B6E`:
   - `G39(f1, c0)`; `G21(0, f2, c0)`, `G21(1, f2, c0)`; `G29(0, f2, c0)`, `G29(1, f2, c0)`;
   - `G4(0)`; `G5(rgb(0xDA, 0xE0, 0xE5))`; `G8(1, 1)`; `G9(rgb(0xAE, 0xBF, 0xC0))`; `G10(rgb(0xEF, 0xF3, 0xF7))`;
   - `G18(c1)`; `G31(c1)`; `G30(0, 1)`; `G37(&title, 0)`.
9. By kind:
   - **bar** (§4.1), **line** (§4.2);
   - **pie**: nothing more, so a pie graph is never filled *[quirk]*.
10. Show the widget (slot 69); free the title; release the exemplar.

### 4.1 Bar set-up

1. `G12(&{45, 20, w − 2, h − 20})` (the plot area, from the panel area's width `w` and height `h`).
2. `B5(1)`. `n` = the number of keys; resize the records to `n`; `+0x48` := 1.
3. Each record `i`:
   - key; label `text(labels[i])` also given to the widget (`B12(&label, i)`), or empty;
   - scale (default 1.0); group (default 0); operation (default 0);
   - `+0x48` := max(`+0x48`, group + 1).
4. `+0x48` := min(`+0x48`, 16) and `B5(+0x48)`.
5. Each group `j`: fill colour `B15(rgb(fill[3j..]), j)`, default (0xAA, 0xBB, 0xC3); edge colour
   `B16(rgb(edge[3j..]), j)`, default (0x67, 0x71, 0x76).
6. Reset `+0x50`/`+0x54`/`+0x58` to unset, then read `0xEA4AEDC6` → `+0x54`, `0x6A4AEDED` → `+0x50` and
   `0x6A4AEDEF` → `+0x58`.
7. `RefreshGraph`; release `B`.

### 4.2 Line set-up

1. `G12(&{45, 20, w − 110, h − 20})`. `f3` = `RZ::WCM` slot 5 `(0xE9C86B5E)`; QI `L`; `G23(0, 1)`; `G32(0, 0)`.
2. Resize the records to the number of keys. Each record `i`:
   - key, second key and operation (each default 0);
   - label `text(labels[i])` or empty;
   - colour `colours[3i..3i+2]` or (0, 0, 0); scale (default 1.0);
   - visible = the visibility map's value for the key, or 1 when absent.
3. **Legend**, from `y = 20`, at `x = w − 106` (`w − 0x6A`):
   1. With **more than 2** series, a checkbox (`cGZWinBtn`, its look from the UI script key `0x856DDBAC`,
      `0x46A006B0`, `0x14416245`) is made:
      - area `(w − 108, y)`–`(w − 92, y + 16)` and ID `0x04000000 + i`;
      - the main window as notification target; state = visible (slot 5 `(1, visible)`); sound `0x4A5C31D7`;
      - added to the widget.

      Then `x = w − 90`.
   2. A swatch (`G51`):
      - rect `(x, y+3)`–`(x+10, y+9)`;
      - fields `+0x18` = 1, `+0x1C` = 1, `+0x24` = 1;
      - edge `c1`, fill = the series colour.
   3. A text item (`G49`):
      - the label in font `f3`, colour `c0`, laid out in `(swatch x1 + 4, y)`–`(w − 4, y + 10)`;
      - `y` += its height + 4.
4. The axis range as §4.1 step 6. `RefreshGraph`; release `L`.

## 5. `RefreshGraph` (`0x0076BE70`) *[compat]*

Nothing without a widget. By kind: 2 → §5.1, 1 → §5.2, other → nothing.

### 5.1 Bar

1. QI `B`. `today` = `sim` slot 8. A second date `today − 60` is made and never used *[quirk]*.
2. Up to 16 group vectors, each `+0x44` floats (0).
3. For each record `i`:
   - `v = H.Get(key, today)` (0 on failure);
   - **operation 5** with both the maximum `M` and the minimum `m` set (§5.3): `v := 0.5 × M × (ln v / ln M + v / M)`
     for `v > 0`, `v := 0.5 × m × (ln(−v) / ln(−m) + v / m)` for `v < 0`, unchanged for 0;
   - `vector[group][i] := v × scale`.

   A series fills only its own group's vector.
4. `B21(&vector[j], j)` for each group.
5. The axis (§5.3), with the automatic maximum `G34(1)` and minimum 0. Release `B`.

### 5.2 Line

1. QI `L`; `(N, S)` = the time scale. `L8()`; `L5(+0x44)`.
2. **Visibility:**
   - for each record `i`, the widget's child `0x04000000 + i` (a legend checkbox), when present, gives its state
     (slot 4 `(1)`); the record and the map remember it;
   - without a checkbox the series is visible;
   - each visible series gets a line `k` (counting visible series only): `L14(&style, k)`, with the style record
     (`0x0076BD20`): byte 0 = 0, byte 1 = `0x41`, `+4` = 12, byte 8 = 1, `+0xC` = the series colour, `+0x10` = 1,
     and an empty string.
3. **Dates:**
   - `start` = `sim` slot 5 (`GetSimStartDate`);
   - `a` = `today − 4`, rounded **up** to a multiple of `S`, minus `(N + 1)·S`.
4. **Samples:** `N + 1` times:
   1. `c = a + S`.
   2. For each visible series:
      - when `start < a` (the period began after the city's start): `v = H.Get(key, c)`, and with an operation
        `v2 = H.Get(key2, c)` (0 on failure); otherwise both are 0;
      - by the operation: 1 `v + v2`, 2 `v − v2`, 3 `v × v2`, 4 `|v2| > 1e−7 ? v / v2 : 0`, other values unchanged;
      - `L20(v × scale, k, −1)`.
   3. `a := c`.

   So the last sample is at `today − 4` rounded up.
5. `L19()`. The axis titles:
   - `G17(text(label + 1), 0)`, the time scale's second string;
   - `G17(text(0x8A5D420D), N)` *[inferred: the second argument; GW2]*.
6. The y axis (§5.3), with the automatic maximum `G25(1)` and minimum `G26(1)`; a positive automatic minimum becomes
   0. Then `G15(0, 0)` as well.

### 5.3 The y axis

1. `max` = `+0x50` when set, else automatic. `min` = `+0x54` when set, else as above. `r = trunc(max − min)`, as
   unsigned.
2. **Step:**
   - `r < 1`: `max = 1.0`, step 0.2, `G27(1, 1)`;
   - `r < 3`: `max = ceil(max)`, step 0.5, `G27(1, 1)`;
   - otherwise:
     1. `u = 1`; while `r ≥ 100`: `u = trunc(u × 10)`, `r = (r + 9) / 10`.
     2. The step is `10u` for `r > 50`, `5u` for `r > 20`, `4u` for `r > 10`, `2u` for `r > 5`, else `u`.
     3. Without a set maximum, `max ×= 20/19`. `G27(1, 0)`.
3. A set step (`+0x58` ≠ 0) replaces it.
4. `G15(1, min)`, `G16(1, max)`, `G24(1, step)`.

## 6. The widget slots used

All on `G` unless marked; none of these widgets is specified yet (GW1). `G4` (`+0x10`), `G5` (`+0x14`), `G8`
(`+0x20`), `G9` (`+0x24`), `G10` (`+0x28`), `G12` (`+0x30`, the plot area), `G15`/`G16` (`+0x3C`/`+0x40`, axis min and
max), `G17` (`+0x44`), `G18` (`+0x48`), `G21` (`+0x54`), `G23` (`+0x5C`), `G24` (`+0x60`, the step), `G25`/`G26`
(`+0x64`/`+0x68`), `G27` (`+0x6C`), `G29` (`+0x74`), `G30` (`+0x78`), `G31` (`+0x7C`), `G32` (`+0x80`), `G34`
(`+0x88`), `G37` (`+0x94`, the title), `G39` (`+0x9C`), `G49` (`+0xC4`), `G51` (`+0xCC`); `B5`, `B12`, `B15`, `B16`,
`B21`; `L5`, `L8`, `L14`, `L19`, `L20`. `cSC4LineGraph` overrides `G0` and `G43` (`+0xAC`, a line draw; §7).

## 7. Helpers *[compat]*

- Series vector machinery (record size 0x30):
  - copy-construct `0x0076BDF0`; uninitialized copy `0x0076C9C0`; uninitialized fill `0x0076C9F0`;
  - destroy range `0x0076CA20`; copy backward `0x0076CCC0`; fill `0x0076CD70`; copy `0x0076CE00`;
  - grow-insert `0x0076CEB0` (capacity `size + max(size, n)`); insert `n` `0x0076D150`; resize `0x0076D2F0`;
  - resize with an empty record `0x0076D370`; clear `0x0076D2B0`.
- The visibility map: insert with hint `0x0076BB10`, `operator[]` `0x0076BD80` (a new key gets 0).
- `0x0076BD20`: the line style record's default (§5.2).
- `0x0076B5D0`: the child walk's callback (§1).
- The line widget's own code, attributed here by reach:
  - `0x0076B5F0` is the `cSC4LineGraph` constructor: `cGZLineGraph`'s, then the vtables `0x00AB4D08` (`+0`),
    `0x00AB4C28` (`+0xD8`, the graph interface) and `0x00AB4B98` (`+0x22C`);
  - `0x0076B840` is its deleting destructor (`0x009BAA36`, then free when bit 0 of the flag is set);
  - `0x009BAA36` is the `cGZLineGraph` destructor body (vtables, its shutdown `0x009B5E8B`, three containers, then
    `cGZGraph`'s);
  - `0x009B5E8B` releases the widget's `+0x258` and tail-calls `cGZGraph::Shutdown`.
- **`cSC4LineGraph`'s two overrides**, the reason for the subclass:
  - `0x0076B660`, `G43` (`0x00AB4CD4`), RET `0x1C`: `DrawLineAntialiased(a1, …, a7, 0)` with its seven arguments
    in order. So the panel's line graphs are drawn antialiased.
  - `0x0076B6B0`, `G0` (`0x00AB4C28`, also `0x00ADF208`): `this − 0xD8`, then `cGZLineGraph::QueryInterface`.

## 8. Quirks and cases

Quirks:
- Pie graphs get no data.
- The bar refresh makes an unused date.
- Only the low byte of a `0x4xxx` button selects the graph.
- A series fills only its own bar group.
- Line samples before the city's start date are 0.
- The legend has no checkboxes for one or two series, so those series cannot be hidden.

| Case | Setup | Expected |
|---|---|---|
| GW-C1 | time scale 0 on day 1000 | `a` = 996 rounded up to 1020, minus 13 × 30 = 630; samples at 660, 690, …, 1020 |
| GW-C2 | bar Demands, `v` = 3000 | `0.5 × 6000 × (ln 3000 / ln 6000 + 0.5)` ≈ 4261.0 |
| GW-C3 | axis range 250, no set maximum | `u` = 10, `r` = 259 / 10 = 25 → step 50; `max ×= 20/19` |
| GW-C4 | operation 4 with `v2` = 0 | 0 |
| GW-C5 | button `0x4105` | graph 5 selected |

## 9. Function inventory

Owner `cSC4LineGraph`: both functions (59 bytes) in §7 (`0x0076B660`, `0x0076B6B0`).

Owner `cSC4GraphsViewWinProc`: all 35 functions (11,633 bytes) are specified: §1 (`0x0076B5D0`, `0x0076B750`, `0x0076CA60`, `0x0076CFE0`,
`0x0076D080`, `0x0076D290`), §3 (`0x0076B860`, `0x0076B8B0`, `0x0076C910`, `0x0076CC80`, `0x0076E420`, `0x0076E470`,
`0x0076E4B0`, `0x0076E500`), §4 (`0x0076B5F0`, `0x0076D3D0`), §5 (`0x0076BE70`), §7 (`0x0076B840`, `0x0076BB10`,
`0x0076BD20`, `0x0076BD80`, `0x0076BDF0`, `0x0076C9C0`, `0x0076C9F0`, `0x0076CA20`, `0x0076CCC0`, `0x0076CD70`,
`0x0076CE00`, `0x0076CEB0`, `0x0076D150`, `0x0076D2B0`, `0x0076D2F0`, `0x0076D370`, `0x009B5E8B`, `0x009BAA36`).
