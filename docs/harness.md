# Testing your DLL in the game (for lanes)

The coordinator runs a **game queue**: it watches every worktree for request files, runs each request in the real
game with that worktree's `FreeCity4.dll`, and writes the result back into the same worktree. Several games run at
once (one per *slot*, default 3), each with its own copy of the test region, so your run does not disturb anyone
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
| `no-load` | the city never loaded |
| `timeout` | no verdict before `timeout` |
| `rejected` | the request was not run; `line:` says why (bad label, unknown key, DLL outside your worktree, no such city or file) |
| `harness-error` | the harness could not run it (a slot was busy, a file was locked); send it again |

## How long to wait

A run takes about 10 s to start and load the city, then `seconds` (or until `end_date`; turbo does about 50 to 100
game days per second), then a few seconds to quit. If other lanes' runs fill the slots, yours waits for a free one;
the `.running` file appears when it starts. Check for the `.verdict` file every 30 s or so; if nothing has happened
after `timeout` plus 15 minutes, ask the coordinator (the queue may be stopped).

## What a slot run can and cannot do

- There is no keyboard or mouse: the game runs minimized and is never brought to the front. No UI-driven tests, no
  Ctrl+S. Drive the city with `script = <file>` actions instead (including `save`).
- Game dialogs (advisor popups and the like, drawn by the game) are not clicked away. Keep `popup_dialogs = 0` (the
  queue adds it). If one still holds the city, the verdict file has a `held:` line ("held by a dialog since <game
  date>"), and the run usually ends at its timeout.
- Windows message boxes (class `#32770`, e.g. an assert or runtime error box; no plugin code runs while one is open)
  are read and closed from outside: OK if there is one, else Cancel, else WM_CLOSE (a Yes/No box stays open and is
  reported). Each gives a `dialog:` line with its title and text. One whose title says "Error" before the plugin
  has attached (e.g. "Hardware Driver Error") is a `failed-start`.
- Each slot starts from a fresh copy of the test region, so a city changed or saved by your run does not carry over
  to the next run.
- Do not start the game yourself and do not write to `Documents\SimCity 4\`; the queue owns the game.

## For the coordinator

Start the queue detached from the main checkout and leave it running (one queue at a time; a second one exits):

```
Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File C:\Users\tlori\FreeCity4\tools\game_queue.ps1 -Slots 3"
```

It logs to `build-runs\game-queue.log` of that checkout. `-Once` runs what is queued and exits; `-Roots a,b` scans
only those checkouts. Slot games live in `build-runs\sc4slots\slotN` of the main checkout (the game ignores a
`-UserDir` with a path component that starts with `.`, such as `.claude`, and falls back to Documents). Stopping the
queue does not stop the games it started; their runs end at their timeout. `regress.ps1 -Parallel` uses slots 1
and 2; the queue leaves a slot alone while another harness run holds it, but a regress run whose slot the queue holds
waits only 60 s and then ends BLOCKED, so start a regress while the queue is idle.
