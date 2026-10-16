tag: cores12a-123
run: 123
date: 2026-10-16
lane: cores12a
dll: FreeCity4-cores12a
pacing: turbo
verdict: FAIL
switches: main `cd3c5b0` (cores12a: demolition utility, dispatch manager and city situation manager cores (`engine/core/src/{demolition,dispatch,city_situation}/`), capi `fc4_demolition`, `fc4_dispatch`, `fc4_citysit` and their `dm*` / `dp*` / `cs*` vectors; **no plugin code**; ABI 58, taken as 60 on the lane; DLL `FreeCity4-cores12a`, sha256 `3c77bd13`), `tools/regress.ps1 -Tag cores12a -Queue` (turbo to 2032-01-01, lists as run 121), run twice
result: **FAIL, reverted (`dff8e4e`).** All four games (slots 1 and 2, both attempts) stay in the region view: `autoload: the region lists no cities yet` ten times within ~0.13 s (region slot 23 GetCityLocations returns an empty list), then `giving up`; `no city load within 1950 s`, no verdict, 0 problems. A/B in the same queue minutes later (`ab-autoload-abitidy` / `ab-autoload-cores12a`, minimal ini, turbo to 2026-03-01): the abitidy DLL (main `b435187`, run 122) loads Konradshohe and ends clean in 16 s; the cores12a DLL fails the same way in slot 6. abitidy2's lane DLL, which has this merge, fails its smoke runs the same way in slot 3; every DLL without it loads in every slot. The merge only adds Rust cores, capi modules and fc4.h declarations (the DLL grows by 3.5 KB), so the mechanism is not yet known. Gate cores12a was clean (cargo 2221/0, clippy clean, ctest 55/55).
summary: regress cores12a, FAIL: no city load; merge reverted
