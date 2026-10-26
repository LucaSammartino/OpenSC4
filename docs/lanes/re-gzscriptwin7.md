| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzscriptwin7: the game's scripted-window classes (`cSC4ScriptedWindowCOMDirector`, `cSC4WinAlertBorder`, `cGZWinKnob`), scripted-window-director.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzuiscript7b) | **ready** (docs only). The director's getter and 17-class table, the constructors it holds, the alert border and the knob; owners claimed: specified +4,418 bytes (+59 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 59 functions were read in the decompile; the director's constructor and getter, every factory, the map-view
constructor, the alert border's `GZPaint` and the knob's frame choice and `QueryInterface` were checked on the
instructions. The vtables `0x00A87F58`, `0x00AB5B20`, `0x00AB5B48`, `0x00AB4900` and `0x00AB64B8` were read. Written on
2026-10-24.

### What changed

- New docs/specs/scripted-window-director.md: the director (§1), its class table (§2), the constructors it holds (§3),
  the alert border (§4), the knob (§5), quirks (§6), inventory (§7).
- widget-toolkit.md coverage-status rows point to it.

### Quirks found

- Classes `0x8918C708` and `0xE918C70C` are registered with a factory that returns null, and class lookup calls the
  result's `QueryInterface` unchecked.
- The game's IME candidate list reuses the GZ class ID `0x2AC45173`; this director is added before the GZ window
  director, so lookups get the subclass.
- The alert border paints only while a settings byte is set; its picture is a 3 × 3 edge grid.
- The knob's frame 0 is its disabled picture; values map to frames 1…N−1.

### Coverage

- tools/coverage-map.tsv: the `cSC4ScriptedWindowCOMDirector` row now points to scripted-window-director.md; new rows
  `cSC4WinAlertBorder`, the four `fn:GZCOM_cSC4Win*CLSID` getters and `fn:SC4GetSC4ScriptedWindowCOMDirector` (before
  `cSC4Win*`), `cGZWinKnob` (before `cGZWin*`).
- Specified bytes 4,751,213 → 4,755,631 (+4,418, 85.1% → 85.2%, `cov_local.py`).

### Not covered

- The aura bar, RCI bar, trend bar and transparent window beyond their constructors (their other methods belong to
  other owners or are unattributed); the network lot chooser and editor (developer tools).
