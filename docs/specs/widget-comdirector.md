# `cGZWinCOMDirector`: the window library's COM director (behaviour spec)

Scope: the `cGZWinCOMDirector` owner as tools/coverage.py assigns it on `bb641878` (115 functions, 9,760 bytes). The
owner is the director of the GZ window library **plus** methods of several widget classes that the coverage tool
attributed to it by reach. This spec describes the director (§1–§3) and points to the specs that describe the rest
(§4), so that every function of the owner is specified somewhere.

Related: app-directors.md (`cRZCOMDllDirector`: `AddCls`, `AddDirector`, class lookup); system-service-pointer.md;
the widget specs named in §4.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the owner was read in the decompile. Written 2026-10-24 (re-gzcomdir7).

## 1. The director *[compat]*

- **Constructor** (`0x00998B90`): `cRZCOMDllDirector`'s; vtables (the framework-hooks part at `+4`, `0x00ADC220`); adds the
  Win32 window director (`GetGZWin32COMDllDirector()`) as a child (`AddDirector`); then registers the classes of §2 in
  that order (`AddCls`, kind 1: a no-argument factory, app-directors.md §2.3).
- **`GetDirectorID`** (`0x00998DB4`): `0xA41740D1`.
- **`OnStart(com)`** (`0x00998DBA`): the framework (`RZGetFramework`) gets the director's hooks interface (its slot 7);
  then two system services are created through `com` (slot 3, interface `0x287FB697`) and added to the framework (its
  slot 3), each local reference released: class `0x84174448` (registered by the child director) and `0xC2C2EB0F`
  (the window-control manager, widget-ctrlmgr.md). True.

## 2. Class table

Each factory allocates (global heap) and constructs its object; a failed allocation returns null. Some return an
interface inside the object (the offset given).

| Class | Factory | Object, size | Returned |
|---|---|---|---|
| `0xC2C2EB0F` | `0x00998961` | `cGZWinCtrlMgr`, `0x98` | the object |
| `0xAC0874D8` | `0x0099897A` | `cGZWinDragAndDropManager`, `0x108` | `+8` |
| `0x22C010CE` | `0x009D9F40` | `cGZWinToolTipMgr`, `0x4C` (widget-tooltip.md §6) | the object |
| `0x4386D516` | `0x0099B74D` | `cGZWinGen`, `0x128` (widget-wingen.md) | `+0xD8` |
| `0xC2AFA76E` | `0x0099899E` | `cGZWinFlatRect`, `0xF0` (constructor `0x009CD842`) | the object |
| `0x592` | `0x009989B7` | `cGZWinText`, `0x114` (widget-text.md) | the object |
| `0x231A1C58` | `0x009989D0` | `cGZWinTextEdit`, `0x290`, flags `0x11FF` | the object |
| `0x593` | `0x009989EF` | `cGZWinMsgBox`, `0x118` | the object |
| `0x598` | `0x00998A08` | `cGZWinListBox`, `0x168`, argument 0 | the object |
| `0x59B` | `0x00998A24` | `cGZWinCombo`, `0x13C`, argument 1 | the object |
| `0x1AA52EA4` | `0x00998A40` | `cGZWinFileBrowser`, `0x364` (constructor `0x009A57C4`) | `+4` |
| `0x3AE8BAE1` | `0x00998A64` | `cGZWinTreeView`, `0x1C4` (constructor `0x009D2BF0`, §3) | `+4` |
| `0x86CB8B28` | `0x00998A88` | `cGZWinPieMenu`, `0x198` (widget-piemenu.md) | `+0xD8` |
| `0x22ECFC47` | `cGZWinBtn::CreateNewObject` | button (widget-button.md) | |
| `0x82FE68C4` | `0x00998AC5` | `cGZWinBMP`, `0xFC` | the object |
| `0x21325208` | `cGZWinSlider::CreateNewObject` | slider | |
| `0x61325A2E` | `cGZWinScrollbar::CreateNewObject` | the old scroll bar | |
| `0x4B42DA5D` | `cGZWinScrollbar2::CreateNewObject` | widget-scrollbar.md | |
| `0x21335C5A` | `cGZWinLineInput::CreateNewObject` | widget-lineinput.md | |
| `0xA1336CC1` | `cGZWinOptGrp::CreateNewObject` | option group | |
| `0x612CE0C4` | `cGZWinSpinner::CreateNewObject` | spinner | |
| `0x4303E5B6` | `cGZWinOutline::CreateNewObject` | outline | |
| `0xE32F0B31` | `0x00998ADE` | `cGZWinTextTicker`, `0x138` (widget-ticker.md) | the object |
| `0x078E8055` | `0x00998AF7` | `cGZWinFolders`, `0xFC` (widget-folders.md) | the object |
| `0xDAA6B9BF` | `0x00998B10` | `cGZWinGrid`, `0x2E8`, flags `0x19BFF` | the object |
| `0xE2BA00EE` | `0x00998B2F` | a plain `cGZWin`, `0xD8` | the object |
| `0x2AC4516D` | `0x00998B48` | `cGZWinIMEProxy`, `0x68` | the object |
| `0x2AC45170` | `0x00998B5E` | `cGZWinIMEComposition`, `0x2E8` | the object |
| `0x2AC45173` | `0x00998B77` | `cGZWinIMECandidateList`, `0x154` | the object |
| `0x42E967BE` | `0x00998AAF` | `cRZWinKeyAccelerator`, `0x1C` (constructor `0x009D5FD2`, §3) | the object |

## 3. Two constructors in this owner

- **Tree view** (`0x009D2BF0`; widget-controls.md §6.1 gives its vtables): the `cGZWin` part at `+4`, then zeros except:
  `+0x118` = 11, `+0x128` = 16, `+0x130` = 9, `+0x134`/`+0x138`/`+0x13C` = 3, `+0x148` = 44, `+0x170` = 1, the four
  colours `+0x174`…`+0x180` and `+0x124` = `0xDEADBEEF` (unset), a string at `+0x1A4`, a map at `+0xE0`.
- **Keyboard accelerator** (`0x009D5FD2`; widget-toolkit.md §13): reference count 0, initialised byte 0, an empty map,
  `+0x18` = 0.

`0x0046B4F0` is a two-argument `atan2` (`fpatan`), used by the pie menu's hit test.

## 4. Where the other functions are specified

| Addresses | Count | Spec |
|---|---|---|
| `0x009D40BE`–`0x009D5CB6` | 50 | widget-piemenu.md §8 (the pie menu's interface) |
| `0x009D6530` | 1 | widget-ticker.md (constructor) |
| `0x009D6C36`–`0x009D7D14` | 28 | widget-folders.md §4 |
| `0x009D9833`–`0x009D9F40` | 9 | widget-tooltip.md §6.4 (tooltip manager) |
| `0x00998961`–`0x00998DBA`, `0x0099B74D` | 24 | §1–§2 here |
| `0x009D2BF0`, `0x009D5FD2`, `0x0046B4F0` | 3 | §3 here |

Total 115. The tooltip manager's factory `0x009D9F40` is counted with the manager (§2 only cites it).

## 5. Function inventory (§1–§3)

`0x0046B4F0`, `0x00998961`, `0x0099897A`, `0x0099899E`, `0x009989B7`, `0x009989D0`, `0x009989EF`, `0x00998A08`,
`0x00998A24`, `0x00998A40`, `0x00998A64`, `0x00998A88`, `0x00998AAF`, `0x00998AC5`, `0x00998ADE`, `0x00998AF7`,
`0x00998B10`, `0x00998B2F`, `0x00998B48`, `0x00998B5E`, `0x00998B77`, `0x00998B90` constructor, `0x00998DB4`
`GetDirectorID`, `0x00998DBA` `OnStart`, `0x0099B74D`, `0x009D2BF0`, `0x009D5FD2`.
