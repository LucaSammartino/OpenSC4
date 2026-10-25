| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzuiscript7: the UI script service `cGZUIScriptService`, new spec ui-script-service.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzaccelres7) | **ready** (docs only). ui-script-service.md §1–§6: the seven interface tables and the fields, the service slots, window creation with the resolution-specific script group, interpreter setup and tag registration, the script/tag callbacks, the parent stack, lifecycle, quirks; ui-windows.md §5.1's table corrected (slot 4 is `0x0094B210`); owner claimed: specified +2,786 bytes (+46 functions) | the UI-script COM directors next |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 46 functions were read in the decompile; `CreateWindowFromScript`, the script and tag callbacks,
`ExecuteScriptString`, `Init`, `Shutdown` and the parent stack were checked on the instructions. The seven vtables and
the tag-name strings were read. Written on 2026-10-24.

### What changed

- New docs/specs/ui-script-service.md.
- ui-windows.md §5.1: the two `CreateWindowFromScript` functions are slots 3 and 4 of the service table (the earlier
  "slot 4 is not separately named" was wrong).

### Quirks found

- A UI script can have a per-resolution variant: the group is the screen width and height printed as decimal digits
  and read back as hexadecimal.
- `comic9`/`comic10` are registered as tags with no case in the tag callback.

### Coverage

- tools/coverage-map.tsv: new row `cGZUIScriptService` → ui-script-service.md, before `cGZUIScript*`.
- Specified bytes 4,746,296 → 4,749,082 (+2,786, 85.0%, `cov_local.py`).

### Not covered

- The interpreter, scope, container and tag-object classes; the UI-script COM directors.
