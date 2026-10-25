# Seeded player actions

`python tools/scenario_gen.py --seed 7 --size medium --start 2026-01-28 --months 24 --mix default`
prints a dated plugin script. Sizes are 64, 128 and 256 cells. The default mix
interleaves `grow`, `churn` and `edge` each calendar month. Identical arguments
produce identical UTF-8 bytes, without environment, clock or global RNG input.
Months are 1..1200; the resulting dates must fit years 0001..9999.

- `grow`: a land patch and road/street grid, every zone type and R/C/I density
  beside the grid within four months, parallel pipes/power, rail/avenue
  crossings, later zoning and airport/seaport candidates.
- `churn`: rezone the same rectangle, dezone, change network routes and types,
  edit terrain, survey/start fires and riots, save, and change pacing/lighting.
- `edge`: flood corner patches, attempt zoning on water and steep terrain,
  rezone a development site, cross networks at tile borders, price zones over
  the entire tile and individual border cells. The endpoints remain inside
  the selected tile; native validation handles the map edge.

Native action failures are expected evidence, especially for `edge`. Parser
errors are bugs. Terrain edits and disasters can substantially alter a city.
Run only through isolated game slots: **never install these scripts in the
Documents game folder**. `save` saves the isolated copy in place.

Pause/resume, speed 0/2 and turbo off/on are adjacent on the same date so a
future action never waits behind a stopped simulation clock. Saves defer to
a framework tick and may delay subsequent actions by several game days.
Disaster surveys run monthly, but explicit/auto fire and riot starts run on
the final date, after all requested months: native emergency mode can stop
new days until player intervention. The script includes that final date;
set the harness end date to `start + months`. It drains due actions before
quitting, so the disaster calls need no subsequent calendar progress.

## Fallback game matrix

For checkouts without `tools/suite.py`:

```
python tools/scenario_runs.py prepare --tag gen-trial --dll C:/path/to/FreeCity4.dll
python tools/scenario_runs.py run --tag gen-trial
python tools/scenario_runs.py summarize --tag gen-trial
```

Preparation reads the dimensions and simulator date from the four Documents
FC4 Test saves: Konradshohe, Tegel, Spandau and New City (an unestablished
small tile). It writes 3 seeds × 4 mixes × 4 cities, starting the day after
each save, with 24 calendar months per run (customizable to 24..48). DLL
copies, source hashes, scripts and an interface-compatible `suite.tsv` with
`sets=gen` live under `build-runs/suite/<tag>/`. Tags cannot overwrite earlier
evidence. `--cities`, `--seeds`, `--seconds` and `--slots` customize the matrix.

The runner reads the combined shadows profile directly from `regress.ps1`.
Its named queue instance uses the existing request and slot mutexes, with up
to six request roots because the legacy queue serializes each root. The
legacy queue supports only its default FC4 Test region, so it receives no
new region key. Documents is never a destination.

The harness falls back to a read-only native command-line query if Windows
denies CIM. If neither query identifies a live game, it still refuses that
slot; it never assumes an unidentified process belongs to another slot.

`summary.tsv` has the suite interface columns; `actions.tsv` gives completed
run/ok/failed counts by action and scenario. `per_key.tsv` gives the union;
`findings.tsv` records first observed mismatches and exceptions per scenario,
their seed, log line and preceding completed script action. That action is
context, **not proof of causality**. Rerun the full script to reproduce;
minimize separately before claiming a single action caused a mismatch.
Missing completion markers are reported explicitly. Exact shadow counts
exclude resynced lines; absent compare/reach measurements are `-`.

`python -m pytest tools/tests -q` includes 300 generated scripts checked by a
tiny C++ driver against the plugin's own `FC4HarnessLogic.h` parser, plus
independent range, date, seed, mix and clock-progress checks. These tests
require a C++17 compiler (MSVC is discovered automatically on Windows).
