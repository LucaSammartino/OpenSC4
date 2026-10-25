# Testing your DLL in the game (for lanes)

The coordinator's queue runs each worktree's DLL in an isolated game slot and writes results back there.
The [lane prompt](../.claude/workflows/fc4-lane.js) owns request cadence, labels, waiting and acceptance.
<!-- Stable function-compare interface is documented at the end of this file. -->

The coordinator runs a **game queue**: it watches every worktree for request files, runs each request in the real
game with that worktree's `FreeCity4.dll`, and writes the result back into the same worktree. Several games run at
once (one per *slot*), each with its own copy of the test region, so your run does not disturb anyone
else's and nobody else's run disturbs yours. You never start the game yourself.

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
or `argN`; `-` alone means no memory windows (argument/return-only functions).
Argument indexes are zero-based; offsets/sizes are decimal. Access is `r`, `w`, `rw`.
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
