tag: c12a-smoke-1
run: 148
date: 2026-10-16
lane: cores12a
dll: FreeCity4-cores12a
pacing: turbo
verdict: clean
switches: lane cores12a at `3c27ed5c` (main `cfddb45b` merged in; the demolition utility, dispatch manager and city situation manager cores, with `e451878` re-applied); Konradshohe, `turbo = 1`, `end_date = 2027-01-01`, `seconds = 600`; `popup_dialogs = 0`, `development_shadow = 1`, `protection_shadow = 1` (the shadows that report demolitions and dispatches — the three cores themselves have no `enable =` key); DLL `engine\build-x86\FreeCity4.dll`, sha256 `4042b5bf`
result: **clean**, `ref_warnings=0`, rtc=0, overruns=0. **The city loads**, which is the point of this run: `autoload: the region lists 85 cities (GetCityLocations -> true; 0 ms and 0 empty checks before)` then `autoload: LoadCity(...\City - Konradshohe.sc4) -> true`. End date 2027-01-03, 14.813 s real, 23.56 days/s. Shadows exact: development 147/147, protection 48/48. Problems all 0 (foreign exceptions, stalls, reference warnings, panics, structured exceptions, shadow INCOMPLETE, other WARNINGs). Slot 2, wall 31 s.

# Run c12a-smoke-1

In-game run row keyed by tag (see `docs/merge-train.md`). Request `build-runs/game-queue/c12a-smoke-1.req`
in `.claude/worktrees/cores12a`; results `build-runs/game-results/c12a-smoke-1.{verdict,log}`.

| Tag | Switches | Result |
|---|---|---|
| c12a-smoke-1 | lane cores12a at `3c27ed5c` (main `cfddb45b` merged in; the demolition utility, dispatch manager and city situation manager cores, with `e451878` re-applied); Konradshohe, `turbo = 1`, `end_date = 2027-01-01`, `seconds = 600`; `popup_dialogs = 0`, `development_shadow = 1`, `protection_shadow = 1` (the shadows that report demolitions and dispatches — the three cores themselves have no `enable =` key); DLL `engine\build-x86\FreeCity4.dll`, sha256 `4042b5bf` | **clean**, `ref_warnings=0`, rtc=0, overruns=0. **The city loads**, which is the point of this run: `autoload: the region lists 85 cities (GetCityLocations -> true; 0 ms and 0 empty checks before)` then `autoload: LoadCity(...\City - Konradshohe.sc4) -> true`. End date 2027-01-03, 14.813 s real, 23.56 days/s. Shadows exact: development 147/147, protection 48/48. Problems all 0 (foreign exceptions, stalls, reference warnings, panics, structured exceptions, shadow INCOMPLETE, other WARNINGs). Slot 2, wall 31 s. |
