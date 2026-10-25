# Testing your DLL in the game (for lanes)

<!-- Stable function-compare interface is documented at the end of this file. -->

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

`key = value` lines, `#` starts a comment. Plugin settings after `[ini]` are copied into `FreeCity4.ini` (the harness consumes `fixture_city`; the
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
| `minimize` | `1` | `0` keeps the game window on screen after the city load (slot games are minimized otherwise); for runs that must draw frames |
| `timeout` | seconds + 150 | hard limit for the whole run in real seconds; the game is stopped after it |
| `priority` | `normal` | `high`: honoured only for the coordinator's requests ([ops/running-the-game.md](ops/running-the-game.md)); in a lane's request it is ignored and the verdict says so |

For a persisted save fixture, add `fixture_city = C:\absolute\path\to\build-runs\fixtures\name.sc4`
under the request's `[ini]`. The absolute source must be inside the main checkout's `build-runs/fixtures`,
be a DBPF `.sc4`, and is read only. Keep `city` as an existing FC4 Test alias, such as `Konradshohe`:
the harness stages a writable copy at that alias in the isolated slot after mirroring the region. It strips
`fixture_city` before writing the plugin ini and records the copy's SHA256. Documents and the source fixture
are preserved; the next run mirrors the ordinary city again. Direct `ingame_run.ps1` accepts the same ini key
only with `-Slot` and `-City`. This uses the existing queue, with no daemon restart or new request-header key.

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
ticks have passed (`autoload: LoadCity(<save>) -> true`). While the region lists no cities it retries about once a
second and gives up 120 s after the first empty read; the run then ends `no-load`. A `no-load` says nothing about the
DLL under test unless the DLL was built from a main older than the `autoloadfix` merge: rebuild on current main and
send the request again.

## How long to wait

A run takes about 10 s to start and load, then `seconds` (or until `end_date`; turbo does about 50 to 100 game days
per second), then a few seconds to quit; it may first wait for a free slot (the coordinator's requests go first).
Wait for the verdict in **one** blocking call, never a polling loop:

```
python <MAIN>/.claude/lane-bin/qwait.py <WT>/build-runs/game-results/<label>.verdict
```

It waits up to 9 minutes and prints the first lines; on `timeout`, run the same command again. If there is still no
verdict after the request's `timeout` plus 15 minutes, the queue may be stopped: return state `blocked` with the
label. Tooling lanes may use `python tools/lanes.py wait --verdict <label>` instead.

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

## Stable function compare interface (run 7)

The compare harness is default-off. CMake compiles `engine/plugin/compare/*.tsv`
into entry shells; adding a binding needs one row and its Rust adapter entry in
`engine/capi/src/compare.rs`, with no plugin C++ edit. Registry header, in order:

```text
name address class convention this stack_arg_dwords callee_pop ret windows rust_entry mode
```

Columns are tab-separated. Address is eight lowercase hex digits (preferred VA,
1.1.641 only). Supported conventions: `thiscall` (`this=ecx`), `cdecl`, `stdcall`
(`this=none`); cleanup must equal argument bytes, or zero for cdecl. At most 16
stack dwords; cdecl currently supports at most four. Returns: `void`, `eax`, `al`,
`x87-f32`. `mode` is `compare`, `record` or `replace`. `rust_entry` selects a Rust
adapter, not an original-code address. `docs/abi/functions.tsv`, when present,
supplies sizes and validates concrete entry facts. Until then published spike
CSV sizes are used; zero means unpublished, never inferred from address spacing.

Windows are separated by `;`: `base+offset/size/access/fields`. Bases are `this`
or `argN` (zero-based); offsets/sizes are decimal. Access is `r`, `w`, `rw`.
Size may be `spanC,S,T`: `(args[C]-1)*args[S]+T`, bounded to 4096 elements and
stride, with stride >= T. Example: `arg1+0/span0,2,12/r/0:f32!:3@2` captures
three normal floats per element, with count arg0 and stride arg2. Fields are
comma-separated `offset:type:count`; `@N` repeats at stride argN for a span's
count. Types: `u8`, `u32`, `i32`, `flags`, `f32`, `image`; `all` covers a whole
contiguous window. `!` marks fields safe to perturb. `-` declares no value fields.
Unspecified bytes are zero in persisted data, preserving neither padding nor
pointers. Image fields store RVAs; non-image pointers there refuse persistence.
One window per base, no output aliases; all windows must be private writable
committed memory, never image storage. Scalar stack words without windows are
values; pointer arguments must have windows. Adapters must validate their domain.

| Ini key | Meaning (all activity requires a selection) |
|---|---|
| `compare = names,class:<Class>,all` | Comma-separated names/selectors; empty disables everything. `inplace` is the selection alias, used when compare is empty. |
| `compare_mode = compare/record/replace` | Optional override of row modes; empty uses the registry. Replace is an operator assertion that the selection has matched. Refused calls fall back to original. |
| `compare_record = 1` | Persist bounded inputs while comparing; record mode always persists and never calls Rust on natural calls. |
| `compare_record_limit = 50` | Maximum records per function across runs, clamped 1..1000. Use a fresh corpus directory to start over. |
| `compare_corpus = 1` | After city load, compare every stored input of installed functions in isolated copies. |
| `compare_dense = N` | 0..1000 nearby variants per stored input. Changes one or two marked scalar fields or scalar arguments; count/stride arguments stay fixed. |
| `compare_case_limit = 5` | First K differing inputs per function per process, 0..100, appended as value-only JSONL. Includes labelled deliberate mutations. |
| `inplace_fixture = 1` | Retained Rust-authored hand fixtures; separate source counters. |
| `inplace_mutate = selectors` | Flip one output bit after an exact comparison per function/source; log detection and location. Never install the mutation in live state. |

Natural calls sample first 50 then one in 20; replacement processes every call.
Compare leaves native memory/register/x87 results in the game. Exact compares
every captured byte and the declared return channel; unspecified volatile
registers and x87 exception flags are preserved, not credited as algorithm output.
Confinement watches net changes in containing private regions (64 MiB cap),
excluding active stack. It misses undone writes, unrelated allocations, image
globals; concurrent writes are unattributed. Outside/refused samples earn no match.

The stable summary is one line per selected function/source at report time:

```text
inplace summary name=<n> rva=<6-8 hex> bytes=<n> installed=<0|1> source=<natural|fixture|corpus|dense> calls=N sampled=N compared=N exact=N float_only=N different=N outside=N refused=N nested=N mutation=N detected=N max_ulp=N
```

Only trailing fields may be added: currently `recorded=N replaced=N`.
float_only and different are exclusive mismatches. Read the last report per key.
`build-runs/compare-corpus/<name>.bin` is little-endian records: four u32 header
words (magic 0x43344346, version 1, contract hash, payload bytes), followed by
arg-count, 16 scalar argument words (pointer words zero), window-count, then each
window's byte-count and schema-filtered bytes. Each record <=16 MiB; contract,
sizes and schema are validated on load. Return addresses/code bytes are absent.
`build-runs/compare-cases/<name>.jsonl` holds inputs, both outputs, declared return
values, difference location and image-offset field locations. Paths refer to the
building worktree, even when the queue stages the DLL elsewhere. Convert cases
with `engine/tests/compare_case_to_rust.py --output engine/tests/vectors/<name>.rs`;
see `docs/ops/compare.md` for the lane workflow and complete command.
