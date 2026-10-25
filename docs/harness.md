# Testing your DLL in the game (for lanes)

The coordinator's queue runs each worktree's DLL in an isolated game slot and writes results back there.
The [lane prompt](../.claude/workflows/fc4-lane.js) owns request cadence, labels, waiting and acceptance.

## Submit a request

Build your DLL, then write `build-runs/game-queue/<label>.req` in one go or rename a temporary file.
The queue picks it up after 2 s.

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

Requests from one worktree run in submission order; label/retry rules are in the prompt.

## Request format

`key = value` lines, `#` comments; `[ini]` settings go into `FreeCity4.ini` (switches: `engine/README.md`).

| Key | Default | Meaning |
|---|---|---|
| `label` | the file name | names the results; letters, digits, `.`, `_`, `-`, at most 64 |
| `dll` | `engine/build-x86/FreeCity4.dll` | worktree-relative DLL; outside paths refused; adjacent `.pdb` copied too |
| `city` | `Konradshohe` | FC4 Test alias; has water, unlike `Spandau` |
| `seconds` | 150 | loaded-city runtime (`autotest_seconds`), or deadline for `end_date` |
| `end_date` | - | `YYYY-MM-DD`: stop at this game date (`autotest_end_date`); Konradshohe starts on 2026-01-27 |
| `turbo` | 0 | `1`: `sim_turbo = 1`, unpaced days after first ConnectionsReady |
| `turbo_from` | - | `YYYY-MM-DD`: script action starting turbo at this date |
| `minimize` | `1` | `0`: visible after city load for frame tests |
| `timeout` | seconds + 150 | whole-run hard limit in real seconds |
| `priority` | `normal` | `high`: coordinator only; ignored/noted for lanes ([operations](ops/running-the-game.md)) |

For fixtures, put `fixture_city = C:/absolute/path/to/build-runs/fixtures/name.sc4` under `[ini]`.
It must be a DBPF `.sc4` inside the main checkout's `build-runs/fixtures`. Keep `city` an existing FC4 Test alias.
The harness stages a writable copy there, strips the key from the plugin ini and records SHA256; source and
Documents are preserved. The next run remirrors the ordinary city. Direct `ingame_run.ps1` requires `-Slot` and `-City`.

Use `/` or literal single backslashes; escape processing of `\b`, `\t`, `\n` creates rejected control characters
(shown as `<0x08>`). Encode as UTF-8/ASCII, or UTF-16 with BOM. Read back if the writer processes escapes.

`autoload_city` is ignored; `city` sets it. `script = <path>` is worktree-relative ("Scripted actions" in
`engine/README.md`: zone, dezone, speed, turbo, save, digest, mark). Unless overridden, the queue adds
`popup_dialogs = 0`, `autotest_unpause = 2`, `autotest_end = quit` for full Shutdown.

## What comes back

In your worktree, under `build-runs\`:

- `game-results\<label>.running`: slot and start time.
- `game-results\<label>.verdict`: first line `verdict: <word>`; `line:` has the full `AUTOTEST VERDICT` counters.
  `held:` diagnoses a city dialog; `dialog:` records Windows boxes and closure. Request, DLL, slot and wall time
  identify the run; `harness:` holds heartbeat/month/pid/slot; `summary:` holds problem, month and shadow counts
  (`<name> N lines, M with every mismatch count 0`). Check every counter, including `ref_warnings`.
- `game-results\<label>.log`: plugin log; `game-results\<label>-*.png`: failure screenshots.
- Completed requests move to `game-queue\done\`.

Verdict words:

| Word | Meaning |
|---|---|
| `clean` | no foreign exception, stall or reference warning; still check summary |
| `corrupt` | a first-chance access violation outside FreeCity4.dll |
| `stalled` | the main thread stopped moving (watchdog) |
| `ended` | the game ended the city itself |
| `failed-start` | game did not start or plugin did not attach; diagnose, then use a new label |
| `exited` | process ended without verdict; `0xC0000409` is Rust panic; inspect log |
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
- Windows boxes (`#32770`, which suspend plugin execution) are read/closed by the harness: OK, else Cancel,
  else WM_CLOSE; Yes/No remains open/reported. `dialog:` records title/text; "Error" before attach is `failed-start`.
- Each slot starts from a fresh copy of the test region, so a city changed or saved by your run does not carry over
  to the next run.
