# Run glue17-shadows-1

In-game run row keyed by tag (see `docs/merge-train.md`). Request `build-runs/game-queue/glue17-shadows-1.req`
in `.claude/worktrees/glue17`; results `build-runs/game-results/glue17-shadows-1.{verdict,log}`.

| Tag | Switches | Result |
|---|---|---|
| glue17-shadows-1 | lane glue17 at `08aee183`+ (cores12a merged in; the three read-only shadows, no `enable =` yet); Konradshohe, `turbo = 1`, `end_date = 2032-01-01`, `seconds = 600`; `citysit_shadow = 1`, `demolition_shadow = 1`, `dispatch_shadow = 1`, `popup_dialogs = 0`; DLL `engine/build-x86/FreeCity4.dll`, sha256 `fb7f09a6` | **clean**, `ref_warnings=0`, rtc=0, overruns=0. Date 2032-01-19, 25.203 s real, 87.64 days/s. Shadows: demolition **74/74 exact**; citysit 74 lines with 1 difference each (and 2 from 2027-01, when the original tracks a vehicle); dispatch 74 lines with 2 differences each. All three differences are state a record-only shadow does not own: `maxCSI` (city-situation.md §2: a process-wide static `0x00B21D34` the automata manager loads from its own tuning, not a member), `tracked` (the indicator tracking of §4.2, built from the AdviceAdded / AdviceRemoved messages the shadow does not follow) and `availPylons` (dispatch.md §4.5: the free units of the sources' stations, filled into the tool's pylon list by BeginDispatchTool). Both records round-tripped byte for byte (7 and 22 bytes) on every line. Problems all 0. Slot 2, wall 41 s. |
