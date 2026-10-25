# Testing your DLL in the game (for lanes)

The coordinator's queue runs each worktree's DLL in an isolated game slot and writes results back there.
The [lane prompt](../.claude/workflows/fc4-lane.js) owns request cadence, labels, waiting and acceptance.

## Submit a request

1. Build your DLL (`engine/build-x86/FreeCity4.dll` by default).
2. Write `build-runs/game-queue/<label>.req` in your worktree (create the folders if needed;
   `build-runs/` is git-ignored). Write it in one go (write a temporary name, then rename to `.req`, or just write
   it whole): the queue picks up a file once it is 2 s old.

Example:

```
label   = power-check-1
seconds = 120
[ini]
enable = power
power_shadow = 1
popup_dialogs = 0
autotest_end = quit
```

Requests from one worktree run in submission order. A written verdict is final; any retry needs a new label.

## Request format

`key = value` lines, `#` starts a comment. Plugin settings after `[ini]` are copied into `FreeCity4.ini` (the harness consumes `fixture_city`; the
switches are in `engine/README.md`).

| Key | Default | Meaning |
|---|---|---|
| `label` | the file name | names the results; letters, digits, `.`, `_`, `-`, at most 64 |
| `dll` | `engine/build-x86/FreeCity4.dll` | worktree-relative DLL; outside paths refused; adjacent `.pdb` copied too |
| `city` | `Konradshohe` | a city of the FC4 Test region (`Konradshohe` has a water system; `Spandau` has none) |

| `seconds` | 150 | how long the city runs after it has loaded, in real seconds (the plugin's `autotest_seconds`); with `end_date` it is the timeout for reaching that date |
| `end_date` | - | `YYYY-MM-DD`: stop at this game date (`autotest_end_date`); Konradshohe starts on 2026-01-27 |
| `turbo` | 0 | `1`: `sim_turbo = 1`, unpaced days after first ConnectionsReady |
| `turbo_from` | - | `YYYY-MM-DD`: script action starting turbo at this date |
| `minimize` | `1` | `0` keeps the game window on screen after the city load (slot games are minimized otherwise); for runs that must draw frames |
| `timeout` | seconds + 150 | hard limit for the whole run in real seconds; the game is stopped after it |
| `priority` | `normal` | `high`: honoured only for the coordinator's requests ([ops/running-the-game.md](ops/running-the-game.md)); in a lane's request it is ignored and the verdict says so |

For fixtures, put `fixture_city = C:/absolute/path/to/build-runs/fixtures/name.sc4` under `[ini]`.
It must be a DBPF `.sc4` inside the main checkout's `build-runs/fixtures`. Keep `city` an existing FC4 Test alias.
The harness stages a writable copy there, strips the key from the plugin ini and records SHA256; source and
Documents are preserved. The next run remirrors the ordinary city. Direct `ingame_run.ps1` requires `-Slot` and `-City`.

Use `/` or literal single backslashes; escape processing of `\b`, `\t`, `\n` creates rejected control characters
(shown as `<0x08>`). Encode as UTF-8/ASCII, or UTF-16 with BOM. Read back if the writer processes escapes.

In the `[ini]` part, `autoload_city` is ignored (`city` sets it), and `script = <path>` is read relative to your
worktree (a file of dated in-city actions, see "Scripted actions" in `engine/README.md`: zone, dezone, speed, turbo,
`save`, digest, mark). Unless your `[ini]` sets them, the queue adds `popup_dialogs = 0`, `autotest_unpause = 2`
and `autotest_end = quit` (so every run ends through the full Shutdown path).

## What comes back

In your worktree, under `build-runs\`:

- `game-results\<label>.running` while the run is in progress (which slot, since when).
- `game-results\<label>.verdict`: first line `verdict: <word>`; `line:` has the full `AUTOTEST VERDICT` counters.
  `held:` diagnoses a city dialog; `dialog:` records Windows boxes and closure. Request, DLL, slot and wall time
  identify the run; `harness:` holds heartbeat/month/pid/slot; `summary:` holds problem, month and shadow counts
  (`<name> N lines, M with every mismatch count 0`). Check every counter, including `ref_warnings`.
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
| `failed-start` | game did not start or plugin did not attach; diagnose, then use a new label |
| `exited` | the game process ended without a verdict (a crash: `0xC0000409` is a Rust panic; see the log's last lines) |
| `no-load` | the city never loaded (see "City load" below) |
| `timeout` | no verdict before `timeout` |
| `rejected` | not run; `line:` explains validation failure, `the line:` repeats the input; controls shown as `<0xNN>` |
| `harness-error` | slot/locking/harness failure; diagnose, then use a new label |

## City load

Autoload (`engine/plugin/FC4Autoload.h`) waits for region view and 60 framework ticks, then logs
`autoload: LoadCity(<save>) -> true`. An empty city list retries about once a second for 120 s before `no-load`.
Report load failures to the coordinator; they do not establish a subsystem verdict.

## How long to wait

Wait as the prompt directs:

```
python <MAIN>/.claude/lane-bin/qwait.py <WT>/build-runs/game-results/<label>.verdict
```

After the request's `timeout` plus 15 minutes without a verdict, return `blocked` with the label.

## What a slot run can and cannot do

- `minimize = 0` permits frame tests; city actions still use `script`, including `save`.
- `close_dialogs = 1` lets the plugin close game modal dialogs via the default button (fallback `EndModal`),
  logging `dialogs: closed …` and `dialogs_closed=N`. It does not cover bankruptcy, Lua errors or Windows boxes.
  A remaining pause produces `held:` and usually `timeout`; screenshots are diagnostic.
- Windows message boxes (class `#32770`, e.g. an assert or runtime error box; no plugin code runs while one is open)
  are read and closed from outside: OK if there is one, else Cancel, else WM_CLOSE (a Yes/No box stays open and is
  reported). Each gives a `dialog:` line with its title and text. One whose title says "Error" before the plugin
  has attached (e.g. "Hardware Driver Error") is a `failed-start`.
- Each slot starts from a fresh copy of the test region, so a city changed or saved by your run does not carry over
  to the next run.
