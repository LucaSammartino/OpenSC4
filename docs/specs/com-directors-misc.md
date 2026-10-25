# Graphics, draw and Gimex COM directors; the disaster placement tool (behaviour spec)

Scope: the owners `cGZDrawCOMDirector` (6 functions, 270 bytes), `cGZGraphicCOMDirector` (3, 269),
`cGZGraphicWin32COMDirector` (4, 170), `cGXCOMDirector` (6, 241) and `cSC4ViewInputControlEffectDisaster` (7, 698), as
tools/coverage.py assigns them.

Related: app-directors.md and renderer-gdrivers-director.md (`cRZCOMDllDirector`: `AddCls`, `AddDirector`,
`GetClassObject`), gzcom-factories.md (class-ID getters), draw-system.md, gz-graphics.md (graphic system, canvas),
gimex.md (factory, reader, writer), disasters.md (disaster managers and the disaster tools).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the draw and graphic directors' registrations and
`OnStart` bodies were read in the disassembly. Written 2026-10-24 (re-comdirs7).

## 1. Directors *[compat]*

All are `cRZCOMDllDirector`s; each creation function allocates the class and runs its constructor (null on a failed
allocation). `OnStart(com)` runs when the director starts.

| Director (ID) | Constructor | Classes (class ID → creation, size) | `OnStart` |
|---|---|---|---|
| `cGZDrawCOMDirector` (`0x0B2EA939`, `0x00825BE0`) | `0x00825B60` | draw context `0x0AE6320E` → `0x0082CEF0` (200 bytes, `0x0082C9A0`); draw system `0xAB2EA3AA` → `GZCOM_CreatecGZDrawSystem`; draw context as buffer `0xAB300B20` → `0x0082B820` (`0x4C` bytes, returned `+0xC`); the font renderer → `GZCOM_CreatecGZDrawFontRenderer`; buffer `0xC470D325` → `0x008298F0` | `0x00825BF0`: creates the draw system as a system service (IID `0x287FB697`) and adds it to the framework (`AddSystemService`), then releases its reference |
| `cGZGraphicCOMDirector` (`0x246B917A`, `0x00989830`) | static `0x009898E0` (first call constructs; destroyed at exit) | adds `cGZGraphicWin32COMDirector` as a child; alpha blitter `0x63260361` → `cGZAlphaBlt` | `0x00989840`: the same for the Win32 graphic system (`0xA499932C`) and then the Win32 canvas (`0x445FABBB`) |
| `cGZGraphicWin32COMDirector` (`0x446B928F`, `0x00989D90`) | static `0x00989DC0` | `0xA499932C` → `cGZGraphicSystemW32` (`0x180` bytes, `0x00989D50`); `0x445FABBB` → `cGZCanvasW32` (`0xB4`, `0x00989D70`) | — |
| `cGXCOMDirector` (`0x43CAC0B9`, `0x0090AD26`) | `0x0090AD97` (deleting destructor `0x0090AD6E`) | Gimex factory `0xA2FFB5D3` → `0x0090AD2C` (`0x38`); reader `0x2A316C02` → `0x0090AD42` (`0x18`, `0x0090C9F6`); writer `0xCA316C55` → `0x0090AD58` (`0x18`, `0x0090CA7B`) | — |

## 2. The disaster placement tool (`cSC4ViewInputControlEffectDisaster`) *[compat]*

An effect-placement tool (`cSC4ViewInputControlEffect`) that starts a disaster where the player clicks
(disasters.md).

- Constructor `(occupant)` (`0x00580220`): the effect and model IDs `0x6A40222A`, `0x4A08C57E`; when an occupant is
  given, its property `0x4A08C702` (a byte) is copied to `+0x8F`.
- `Init` (`0x00580290`): once; the base `Init`, then the disaster manager `0x6A7B4397` from the city's disaster
  layer (city slot 72, layer slot 7) is kept (`+0xAC`), the "ready" flag `+0xA4` cleared and the manager ID `+0xA8`
  stored. `SetDisasterManager(id)` (`0x00580460`): for a non-zero ID the layer's manager is kept and the flag set;
  the ID is stored.
- `OnMouseUpL` (`0x00580370`): the base handling; when it accepts and the tool has a manager and the flag, the
  manager's slot 6 starts the disaster at the clicked cell (the click position converted to integers). True.
- `Shutdown` (`0x00580300`): releases the manager, the second reference `+0xB0` and the effect `+0x94`, then the base.
  Deleting destructor `0x005804B0`; `0x005803D0` is disasters.md's.

## 3. Function inventory

`cGZDrawCOMDirector`: `0x00825B60`, `0x00825BE0`, `0x00825BF0`, `0x008298F0`, `0x0082B820`, `0x0082CEF0`.
`cGZGraphicCOMDirector`: `0x00989830`, `0x00989840`, `0x009898E0`.
`cGZGraphicWin32COMDirector`: `0x00989D50`, `0x00989D70`, `0x00989D90`, `0x00989DC0`.
`cGXCOMDirector`: `0x0090AD26`, `0x0090AD2C`, `0x0090AD42`, `0x0090AD58`, `0x0090AD6E`, `0x0090AD97`.
`cSC4ViewInputControlEffectDisaster`: `0x00580220`, `0x00580290`, `0x00580300`, `0x00580370`, `0x005803D0`,
`0x00580460`, `0x005804B0`.
