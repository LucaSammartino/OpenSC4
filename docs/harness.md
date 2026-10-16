# Testing your DLL in the game (for lanes)

The coordinator runs a **game queue**: it watches every worktree for request files, runs each request in the real
game with that worktree's `FreeCity4.dll`, and writes the result back into the same worktree. Several games run at
once (one per *slot*), each with its own copy of the test region, so your run does not disturb anyone
else's and nobody else's run disturbs yours. You never start the game yourself.

## Submit a request

1. Build your DLL (`engine\build-x86\FreeCity4.dll` by default).
2. Write a text file `build-runs\game-queue\<anything>.req` in **your worktree** (create the folders if needed;
   `build-runs/` is git-ignored). Write it in one go (write a temporary name, then rename to `.req`, or just write
   it whole): the queue picks up a file once it is 2 s old.

Example, a 120 s run of the power replacement with its shadow:

```
label   = power-check-1
seconds = 120
[ini]
enable = power
power_shadow = 1
popup_dialogs = 0
autotest_end = quit
```

Requests from one worktree run one at a time, in the order you wrote them. Use a new label per request (a request
with an existing label overwrites that label's results).

## Request format

`key = value` lines, `#` starts a comment. Every line after `[ini]` is copied into `FreeCity4.ini` as it is (the
switches are in `engine/README.md`).

| Key | Default | Meaning |
|---|---|---|
| `label` | the file name | names the results; letters, digits, `.`, `_`, `-`, at most 64 |
| `dll` | `engine\build-x86\FreeCity4.dll` | your DLL, relative to your worktree (paths outside it are refused); the `.pdb` next to it is copied too |
| `city` | `Konradshohe` | a city of the FC4 Test region (`Konradshohe` has a water system; `Spandau` has none) |
| `seconds` | 150 | how long the city runs after it has loaded, in real seconds (the plugin's `autotest_seconds`); with `end_date` it is the timeout for reaching that date |
| `end_date` | - | `YYYY-MM-DD`: the run ends when the game date reaches it (`autotest_end_date`). Konradshohe starts on 2026-01-27 |
| `turbo` | 0 | `1`: our simulator's turbo clock (`sim_turbo = 1`): days are not paced by real time, about 0.3 s per game month instead of about 2 s. It starts after the city's first ConnectionsReady |
| `turbo_from` | - | `YYYY-MM-DD`: start the turbo clock at this game date instead (a script action; for DLLs older than 0786064) |
| `timeout` | seconds + 150 | hard limit for the whole run in real seconds; the game is stopped after it |
| `priority` | `normal` | `high`: honoured only for the coordinator's requests (see "For the coordinator"); in a lane's request it is ignored and the verdict says so |

Write paths with plain single backslashes (or `/`). A tool or a string literal that reads `\b`, `\t`, `\n` as escapes
turns `build-runs\dll\b\FreeCity4.dll` into a backspace or `build-runs\dll\tract.dll` into a tab; such a request is
rejected with the line and the character shown as `<0x08>`. Write the file as UTF-8 or ASCII (UTF-16 needs its byte
order mark). Read the file back before you rename it to `.req` if your tool escapes strings.

In the `[ini]` part, `autoload_city` is ignored (`city` sets it), and `script = <path>` is read relative to your
worktree (a file of dated in-city actions, see "Scripted actions" in `engine/README.md`: zone, dezone, speed, turbo,
`save`, digest, mark). Unless your `[ini]` sets them, the queue adds `popup_dialogs = 0`, `autotest_unpause = 2`
and `autotest_end = quit` (so every run ends through the full Shutdown path).

A turbo run to a fixed date gives the same number of month and shadow lines every time, which makes runs easy to
compare; for example `turbo = 1`, `end_date = 2028-01-01`, `seconds = 600`.

## What comes back

In your worktree, under `build-runs\`:

- `game-results\<label>.running` while the run is in progress (which slot, since when).
- `game-results\<label>.verdict` when it is done. The first line is `verdict: <word>`, then:
  - `line:` the full verdict line, e.g. `AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0,
    overruns=0, ref_warnings=0, julian=..., date=2027-06-23, real_seconds=150.7, days_per_second=17.50, ...)`;
  - `held:` if a dialog held the city (see below);
  - `dialog:` one line per Windows message box the game opened (title, text, buttons, how it was closed);
  - the request, the DLL, the slot and the wall time;
  - `harness:` the harness lines (the last heartbeat, the last turbo month line, the game's pid and slot);
  - `summary:` problem counts (all should be 0), the month lines and the shadow lines of every subsystem
    (`<name>  N lines, M with every mismatch count 0`).
- `game-results\<label>.log`: the plugin log of the run (search it for your subsystem's lines).
- `game-results\<label>-*.png`: a picture of the game window when a run ended badly (diagnostics only).
- The request file moves to `game-queue\done\`.

Verdict words:

| Word | Meaning |
|---|---|
| `clean` | the plugin's verdict: no foreign exception, no stall, no reference warning; read the summary for the rest |
| `corrupt` | a first-chance access violation outside FreeCity4.dll |
| `stalled` | the main thread stopped moving (watchdog) |
| `ended` | the game ended the city itself |
| `failed-start` | the game did not start properly (e.g. a graphics driver error, or the plugin never attached); usually not your DLL; send it again |
| `exited` | the game process ended without a verdict (a crash: `0xC0000409` is a Rust panic; see the log's last lines) |
| `no-load` | the city never loaded (see "City load" below) |
| `timeout` | no verdict before `timeout` |
| `rejected` | the request was not run; `line:` says which line and why (bad label, unknown key, DLL outside your worktree, no such city or file, a control character in a value, UTF-16 without a byte order mark); `the line:` repeats it and the request text follows, control characters shown as `<0xNN>` |
| `harness-error` | the harness could not run it (a slot was busy, a file was locked); send it again |

## City load

The plugin's autoload (`engine/plugin/FC4Autoload.h`) opens the city once the region view is up and 60 framework
ticks have passed: `autoload: the region lists K cities (GetCityLocations -> true/false; M ms and N empty checks
before)`, then `autoload: LoadCity(<save>) -> true`. While the region's list is empty it reads it again about once a
second on the wall clock and gives up only 120 s after the first empty read (`autoload: the region lists no cities yet
(N); retrying in 1000 ms (M ms since the first check)`, last `autoload: the region lists no cities; giving up after M
ms (N checks)`).

Before the `autoloadfix` merge the autoload also required `GetCityLocations` to return true, but that return value is
not meaningful (`eax-or-void` in `docs/abi/interfaces.tsv`): in some builds it read as false with all 85 locations of
FC4 Test filled in, the autoload logged "the region lists no cities yet" ten times within ~130 ms (the retry counted
ticks, which come about once a millisecond at start-up), gave up, and the run ended `no-load` ("no city load within N
s (region view? needs a click)") with the region view showing the cities (three queue runs of 2026-10-15). The
value is whatever the game's code leaves in the register: in the failing run it stayed false for 120 reads in one
process, so a run that ends this way says nothing about the DLL under test; with an older DLL, rebuild on a main with
the fix.

## How long to wait

A run takes about 10 s to start and load the city, then `seconds` (or until `end_date`; turbo does about 50 to 100
game days per second), then a few seconds to quit. If other lanes' runs fill the slots, yours waits for a free one;
the coordinator's requests (a regression run, for example) take the next free slots before any lane's. The
`.running` file appears when it starts. Check for the `.verdict` file every 30 s or so; if nothing has happened
after `timeout` plus 15 minutes, ask the coordinator (the queue may be stopped).

## What a slot run can and cannot do

- There is no keyboard or mouse: the game runs minimized and is never brought to the front. No UI-driven tests, no
  Ctrl+S. Drive the city with `script = <file>` actions instead (including `save`).
- Game dialogs (advisor popups and the like, drawn by the game) are not clicked away from outside. Keep
  `popup_dialogs = 0` (the queue adds it). With a DLL from main `dba09eb` or later, `close_dialogs = 1` makes the
  plugin itself end each game modal dialog by its default button (fallback `EndModal`), logs a `dialogs: closed …`
  line per dialog and adds `dialogs_closed=N` to the verdict; with it, `popup_dialogs = 1` runs reach their end date
  too (`docs/specs/modal-dialogs.md`; bankruptcy, Lua error boxes and Windows message boxes are not covered). If a
  dialog still holds the city, the verdict file has a `held:` line ("held by a dialog since <game date>"), and the
  run usually ends at its timeout.
- Windows message boxes (class `#32770`, e.g. an assert or runtime error box; no plugin code runs while one is open)
  are read and closed from outside: OK if there is one, else Cancel, else WM_CLOSE (a Yes/No box stays open and is
  reported). Each gives a `dialog:` line with its title and text. One whose title says "Error" before the plugin
  has attached (e.g. "Hardware Driver Error") is a `failed-start`.
- Each slot starts from a fresh copy of the test region, so a city changed or saved by your run does not carry over
  to the next run.
- Do not start the game yourself and do not write to `Documents\SimCity 4\`; the queue owns the game.

## For the coordinator

Start the queue detached from the main checkout and leave it running (one default instance at a time; a second one
exits; see `-Instance` below for a second queue):

```
Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File C:\Users\tlori\FreeCity4\tools\game_queue.ps1 -Slots 6"
```

It logs to `build-runs\game-queue.log` of that checkout. `-Once` runs what is queued and exits; `-Roots a,b` scans
only those checkouts. Slot games live in `build-runs\sc4slots\slotN` of the main checkout (the game ignores a
`-UserDir` with a path component that starts with `.`, such as `.claude`, and falls back to Documents). Stopping the
queue does not stop the games it started; their runs end at their timeout, their results are lost, and the restarted
queue runs those requests again (in other slots while the old games still hold theirs). Restart at a quiet moment:
`-Status` (below) shows what runs.

**Your requests go first.** A request in the main checkout's `build-runs\game-queue\` is taken before any waiting lane
request (a running game is never stopped; yours gets the next free slot). With `priority = high` it also goes before
your other requests and does not wait for them: several run at once, one slot each (the main checkout's normal
requests still run one at a time, in order). `priority = high` in a lane's request is ignored and noted in its
verdict. The queue's start line in the log says `priority: coordinator <root>`.

**Regression through the queue.** `tools\regress.ps1 -Tag <tag> -Dll <dll> -Queue` submits its two runs as
`priority = high` requests (`regress-<tag>-shadows`, `regress-<tag>-replace`), waits for their verdict files, copies
the logs to `build-runs\regress-<tag>-<run>.log` and prints the same blocks, table, delta and verdict line as
`-Parallel`, plus each run's queue line (slot, time waited) and the wall time. The DLL must be inside the main checkout
(`build-runs\dlls\FreeCity4-<name>.dll` is fine); one outside it is copied to `build-runs\dlls\regress-<tag>.dll`
first. A request that gets no slot within `-QueueWait` seconds (default 3600) is taken back and reported BLOCKED.
Against a queue started before the priority change, regress leaves the key out and the two runs go one after the
other. `-Parallel` still starts the two games directly in slots 1 and 2: use it only while the queue is idle (a slot
the queue holds blocks it after 60 s).

**Regress profile.** Since main `c367684` (the power shadow is exact under turbo) regress runs the turbo profile by
default, in both modes: each run goes to `-EndDate` (default 2032-01-01) with `sim_turbo = 1`, so every run gives the
same month and shadow lines. `-NormalSpeed [-Seconds 150]` runs the old timed profile; `-Turbo` is still accepted.
The shadows run includes `development_shadow = 1` and the replace run `development` (our development unit runs about
3x slower per game day than the original, 23-29 game days/s under turbo against 80-90). Since run 121 the shadows run
also has `traffic_shadow = 1`, which makes it the long one (2.5-3.5 game days/s, 10-15 minutes to 2032; the turbo
timeout is 1800 s), and the replace run `traffic`.

**Pacing.** With the original simulators a city run at about 85 game days per real second ends with other income and
funds than at 8-20 days/s (some original work is bound to frames or real time). Shadow lines are unaffected (both
sides run in one process at one pacing), but a comparison across runs (`-Baseline`, digests) needs runs of the same
pacing. Regress records it in its `profile:` line (turbo to <date>, or normal with its seconds; an older `.out` without
the line counts as normal) and the measured days/s per run in the table. The delta picks a baseline of the same
profile only, refuses a `-Baseline` of the other profile (`-AnyPacing` compares anyway, with a warning), and warns
when the end dates differ or a run's days/s differ by more than 2x (a loaded machine slows turbo runs: in the slot
measurements below the same run did 16 to 43 days/s depending on the lanes' builds).

**Queue status.** `powershell -File tools\game_queue.ps1 -Status [-Last 10]` prints, without disturbing the queue, each
slot and what it runs (lane, label, elapsed, game pid), the waiting requests in the order the queue will take them
(lane, label, class, age; "after its root's earlier request" for a lane's later ones), and the last results with
their verdict words. It reads the queue log(s), the `.running` markers and the request folders, so it also works on a
queue started before this change. `-Check <file>.req` prints what the queue would run for one request, or the exact
rejection it would write.

**More slots, more instances.** `-SlotList 4,5` runs in those slots instead of 1..`-Slots`. `-Instance <name>` starts
a second queue next to the default one, with its own mutex (`FC4GameQueue-<name>`), log
(`build-runs\game-queue-<name>.log`) and work folder; `-CoordinatorRoot <dir>` sets which root has priority (default
the main checkout). Instances of this version claim each request and each slot with a named mutex, so two of them can
scan the same roots without running a request twice; a queue started before this change does not claim, so do not
run a second instance on the roots it scans (the queue started now scans every worktree). Give each instance its own
slots. Slot N runs on logical CPU 2N: on this i7-12700K, slots 1 to 7 each get their own P-core (CPUs 2 to 14),
slots 8 and 9 land on E-cores (16, 18) and run slower.

**How many slots.** Measured 2026-10-14 with the queue on slots 1-3 and the lanes building at the same time: extra
games in slots 4-9 (`ingame_run -Slot N`, main's DLL, 16 replacements, turbo to 2032-01-01). Up to 8 games at once:
no failed start in 26 extra games, every verdict clean, at least 44 GB free (a game uses about 400 MB), total CPU
55-87% on average. A game's speed depends on the build load on its pinned CPU far more than on the number of games:
slot 4 did 32 days/s with 2 other games and 33 with 7 others a minute earlier, but 16 days/s during a heavy build;
slots 8 and 9 (E-cores) did 22-26 days/s next to 31-40 on P-cores. Use `-Slots 6` (CPUs 2-12; P-cores 0 and 7 and
the E-cores stay for the builds and the system); 7 is the most that keeps one P-core per game; do not use 8 or 9.
