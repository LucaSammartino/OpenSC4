# Game runs: glue15 (prop / lot configuration / tutorial glue)

One row per queue run of this lane's `FreeCity4.dll` in the real game (docs/harness.md). The verdict word is the
plugin's; `ref_warnings` must be 0 and every shadow line exact, or every difference explained as a numbered
question. Shadow columns are `<lines>, <lines with every mismatch count 0>`.

City is Konradshohe unless noted. The shadow runs (`sh-*`) enable the 27 regression replacements plus
`propmgr_shadow = 1`, `lotcfg_shadow = 1`, `tutorial_shadow = 1`; `tut-1` enables `tutorial` (the replacement) with
no shadows.

| Run label | DLL | Date / end | Verdict | ref_warnings | propmgr shadow | lotcfg shadow | tutorial shadow | Notes |
|---|---|---|---|---|---|---|---|---|
| `glue15-tut-1` | `build-runs/dll-tut1/FreeCity4.dll` | 2028-01-03, turbo to 2028-01-01 | `clean` | 0 | - | - | - | `enable = tutorial` (the replacement class) with the 27 regress replacements; `tutorial Shutdown` on the shutdown line; no shadows on |
| `glue15-sh-1` | `build-runs/dll-sh1/FreeCity4.dll` | 2028-01-02, turbo to 2028-01-01 | `clean` | 0 | 76, 76 | 101, 101 | 50, 50 | the shadows' first full run; all three exact |
| `glue15-sh-2` | `build-runs/dll-glue15-2/FreeCity4.dll` | 2026-08-12, turbo to 2028-01-01 | `stalled` | 0 | 31, 31 | 41, 41 | 20, 20 | watchdog, no OnTick for 5515 ms under turbo with other lanes in the other slots; the lines that ran were exact; **not this DLL** — `glue15-sh-3` ran the same DLL clean and full |
| `glue15-sh-3` | `build-runs/dll-glue15-2/FreeCity4.dll` | 2028-01-02, turbo to 2028-01-01 | `clean` | 0 | 76, 76 | 101, 101 | 50, 50 | the re-run of `glue15-sh-2`; full and exact |
| `glue15-sh-4` | `build-runs/dll-glue15-2/FreeCity4.dll` | 2032-07-21, **normal speed**, 150 s | `clean` | 0 | 238, 238 | 317, 317 | 158, 158 | the normal-speed check (the addendum asks for one when a turbo run might differ); also exact |
| `glue15-sh-5` | `build-runs/dll-glue15-3/FreeCity4.dll` | 2028-01-03, turbo to 2028-01-01 | `clean` | 0 | 76, 76 | 101, **77** | 50, 50 | **regression found here**: the lotcfg `map` line differed on every month (`firstOrderDiff=@28 60000BC0/0A57C154`), because `fc4_lot_config_unload` ran per shadow run and emptied the map the runs share, so each later run rebuilt the catalogue in a different bucket order |
| `glue15-sh-6` | `build-runs/dll-glue15-4/FreeCity4.dll` | 2028-01-02, turbo to 2028-01-01 | `clean` | 0 | 76, 76 | 101, **77** | 50, 50 | **regression found here**: with the unload moved to app shutdown the map was exact again, but the lotcfg `queries` line then differed on every month (families 16/16 and hasBuilding 32/32, e.g. `family 10000008 n=9/0`), because `g_bindings.clear()` per run left the process catalogue holding stand-in tokens that no longer resolved |
| `glue15-sh-7` | `build-runs/dll-glue15-5/FreeCity4.dll` | 2028-01-03, turbo to 2028-01-01 | `clean` | 0 | 76, 76 | 101, 101 | 50, 50 | both regressions fixed (unload at app shutdown, bindings kept for the process); all three shadows exact again |

## What the exact shadow lines mean

`glue15-sh-7` / `glue15-sh-1` / `glue15-sh-3` (turbo to 2028-01-01) and `glue15-sh-4` (normal speed to 2032-07-21):
FreeCity4's `fc4_prop_manager`, `fc4_lot_config_manager` and `fc4_tutorial` cores, each loaded from the original's
own record (its `cIGZSerializable::Write` into memory) and run over a read-only host, answer exactly what the
original classes answer:

- **propmgr** (76 lines): the catalogue (all prop types and family types in bucket order, each type's key and its
  0x1C-byte profile, each family's members, and the catalogue message's `data2` against the type map count), the
  cell and world queries on 32 sampled cells (160 calls), `CreateProp` on 8 sampled prop types (result, class,
  prop type, bounding box), and the 18-byte record byte for byte and reference by reference;
- **lotcfg** (101 lines): the 26-byte record, the configuration map (1909 configurations, bucket order), the family
  set (282), every configuration's object array against `fc4_lot_config_object_array` on its exemplar (**1909
  configurations, 23302 objects**, the trailing foundation object excluded), and the ByFamily / BySize /
  HasBuildingFamily / HasBuildingType / GetObjectIDArray queries;
- **tutorial** (50 lines): the 11-byte record and IsTutorial / the task number / the GUID.

The two regression runs (`glue15-sh-5`, `glue15-sh-6`) are kept in this table because they are what found the two
lifetime bugs; both were clean for `ref_warnings` and both are fixed at `glue15-sh-7`.

## Not covered by any run

`enable = propmgr` and `enable = lotcfg` (the replacement classes) were **not** run: those classes are not written
yet (see `docs/lanes/glue15.md`). The save sweep for record `0x89C48F47` and the city detail class was not reached.
