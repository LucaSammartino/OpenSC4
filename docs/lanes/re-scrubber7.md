| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scrubber7: the terraforming tool `cSC4ViewInputControlTerrainScrubber` completed and claimed; a FreeType function moved out of it (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-budgetui7) | **ready** (docs and RE tables only). city-tools.md §3: the tunables flag callback, `DoMessage` and its interface, the integer-power helper of the echo scale, destruction, and the owner boundary; tools/coverage-libs.tsv: FreeType range end `0x008AE57B` → `0x008AE770`; row claimed: specified +4,797 bytes (+23 functions), denominator −498 bytes (1 function to FreeType) | — |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`) and the
Ghidra export of the owner. Written on 2026-10-23.

### What changed

- docs/specs/city-tools.md §3 (the tool was specified there; its map row was `-`):
  - §3.1: the static-data callback `0x004D8B70`, which clears the read-once tunables flag;
  - §3.2: `DoMessage` `0x004D9AC0` (slot 3 of the `+0x28` interface, vtable `0x00A92A00`, true for every
    message);
  - §3.4: the integer-power helper `0x004D8EF0` behind `1.7^zoom`, the destructor `0x004D9560` and its deleting
    wrapper `0x004D9950`, and an owner-boundary note.
- tools/coverage-libs.tsv: the FreeType range ended at `0x008AE57B`, "before
  cSC4ViewInputControlTerrainScrubber::SetCursorEchoPos". That function is a weak Mac auto-name (score 0.618,
  support 0) on FreeType code:
  - its only caller `0x008A6D50` and its callee `0x008AE16E` (which fills a table of code pointers in
    `0x008ABxxx`) are inside the range;
  - its body fills a table of doubles and shorts;
  - the next function, `0x008AE770`, is the game's `GZCOM_cGZLuaScriptServerCLSID`.

  The range now ends at `0x008AE770`.

### Coverage

- tools/coverage-map.tsv: `cSC4ViewInputControlTerrainScrubber` → city-tools.md.
- Specified bytes 4,376,965 → 4,381,762 (+4,797, 78.4% → 78.5%, `cov_local.py`, on `5c0ff782` + re-budgetui7).
- The denominator went 5,584,579 → 5,584,081 bytes: the FreeType function left the game code.

### Not covered

Nothing in the owner. The rest of the FreeType range is library code (not in the denominator).
