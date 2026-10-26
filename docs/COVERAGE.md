# Coverage metric

`python tools/coverage.py` answers "how much of SimCity 4 have we reimplemented?" with one reproducible number per
status, weighted by the bytes of the original machine code. It writes [coverage-report.md](coverage-report.md);
rerun it after every merge and commit the report. Same inputs, same report, byte for byte, in the main checkout
(`core.autocrlf=true`, CRLF working files) and in an LF worktree alike: text inputs are hashed with CRLF normalised to
LF, the text as git stores it (a UTF-16 file as its UTF-8 text); only the exe is hashed as raw bytes.

This is an RE-side tool: it reads the exe's analysis (function sizes, names, call graph). Engine lanes never read it
or the report's owner lists; they work from docs/specs only.

## What is counted

- **Function list.** Every function of Ghidra 12.1.4's auto-analysis of `SimCity 4.exe` (GOG 1.1.641, SHA-256
  `3bc5c7fe...1a4b`), from `ghidra/export/windows/functions.tsv` (`tools/setup_ghidra.ps1 -Windows`; gitignored, so
  the tool also looks in the main checkout). The weight of a function is its body size in bytes. The tool checks the
  exe hash, the export's recorded hash and the `.text` bounds before it counts anything.
- **Denominator.** Game code: every function in `.text` except the library and compiler buckets of
  `tools/coverage-libs.tsv` (C runtime and STLport, Lua 5.0, FreeType, libpng, libjpeg, zlib, EA sound, GIMEX, EH
  funclets and static-initialisation stubs, DLL import thunks). Those are reported on their own lines, never in a
  percentage: we link or replace libraries, we do not reimplement them.
- **Matching decompilation projects** count a function as done when their compiled C matches its bytes. FreeCity4 is
  clean-room, so nothing is matched byte for byte; instead every function is attributed to an *owner* (a class, or a
  free function) and the owner gets the statuses of its subsystem. The percentages are therefore exact sums under
  stated rules, at class granularity.

## Attribution rules (in this order; the first that applies wins)

1. **library-range.** The entry lies inside a library range of `coverage-libs.tsv`. Ranges come from link order (a
   static library's object files are contiguous) and override any game-looking name inside them.
2. **Names.** `re/names_windows.tsv`, else Ghidra's own non-default name: a library name rule of `coverage-libs.tsv`
   (`library-name`); a folded (identical-code) entry whose names all share one owner, else shared (`name-folded`); the
   class part of the qualified name, or `fn:<name>` for a free function (`name-high`, `name-medium`, `name-low`,
   `name-ghidra` by the name's confidence).
3. **vtable.** A slot target of a class vtable in `docs/abi/vtables.tsv` or `docs/abi/interfaces.tsv` takes that
   class; a target in vtables of several classes is shared (`vtable-shared`).
   3b. **owner-range.** A function still without an owner whose entry lies in a range of `tools/coverage-ranges.tsv`
   takes that range's owner. For game code that link order identifies but nothing names and no edge reaches, such as
   template instantiations reached only through a hub's dispatch table; every range carries its evidence (vtables,
   tables, callers, Mac names). Named functions inside keep their own owner; the code stays in the denominator.
4. **reach.** An unowned function reached only from functions of one owner takes that owner. Edges are direct calls,
   code references (an instruction operand equal to a function entry), function-pointer tables in data (slots walked
   back to a head that code refers to) and vtables installed by code. A *hub*, a function referring to more than 64
   functions (COM directors, Lua and command registration), passes its owner on through calls and vtables only.
   A function reached from library-range code and from no game owner joins that library (`reach-library`).
5. **neighbour.** An unowned run of functions between two functions of the same owner takes it (one object file).
   Rules 4 and 5 repeat to a fixpoint; only then are functions reached from several owners marked shared
   (`reach-shared`), and the loop repeats.
6. **free-neighbour.** A free function lying between two functions of one class joins that class.
7. **thunk / funclet.** A thunk takes its target's owner; a `Catch@` or `switchD_` funclet its parent's.
8. **unattributed.** Everything else. Reported as its own line, with its largest address ranges, so the size of the
   unknown is never hidden.

Shared and unattributed code stays in the denominator and never counts as done.

## Owners to subsystems: `tools/coverage-map.tsv`

Each owner matches the first row whose glob matches (classes, `ns::*`, `fn:<name>`; catch-all rows at the end). A row
whose pattern is `m:<Class>::<Method>` names one function (exact name, as `re/names_windows.tsv` gives it) and wins over
the owner rows: it credits a partial shadow with only the functions it really compares on a game path, so the
owner's other methods keep the owner row's statuses. A row
gives the subsystem, its area (simulation, framework, renderer, ui, audio, region, other) and four things the
statuses are computed from: spec files, core module paths, shadow ini key, replace ini key. The report warns about
missing spec or core files, unknown keys and rows that match nothing; it also lists the biggest owners only a
catch-all row matches, the candidates for a new row.

## Statuses (per map row; a function has its owner's row's statuses)

| Status | Rule |
|---|---|
| specified | the row names at least one spec and every named `docs/specs/<file>` exists; a spec counts only for the classes that are its subject |
| implemented | the row names at least one core path and every `engine/<path>` exists |
| verified | every compared set of the row's shadow key is exact. Two sources, in this order: (1) the key is in the shadows list of `tools/regress.ps1` and every line of that shadow in the regress `.out` read has all mismatch counts 0; (2) else `tools/coverage-shadows.tsv` has the key and every row it records is exact |
| replaced | the replace key is in `$Replace` of `tools/regress.ps1` (or is on by default in `FC4Subsystems.cpp`) and the regress replace run was clean |

`python tools/coverage.py --suite <tag>` adds three suite columns beside these four, without changing their rules.
The tag lives in `build-runs/suite/<tag>/`; `summary.tsv`, `per_key.tsv`, the staged request manifest, raw logs and
per-scenario `.reach.tsv` counters are read and hashed. Missing counters earn no exercised credit. Missing logs,
nonzero mismatch counts and resyncs cannot earn verification credit. Verification follows the measured shadow key;
a later game failure does not erase earlier exact comparisons. Replacement additionally requires a clean completed run.

| Suite status | Rule |
|---|---|
| suite-exercised | implemented function bytes with positive, unskipped original entry counters in at least one scenario |
| suite-verified | exercised bytes whose map row's shadow key has nonempty exact evidence, every mismatch count zero and zero resyncs, in every scenario that ran that key |
| suite-replaced | implemented rows whose replacement key is enabled in at least one replace-profile run and every run enabling it is clean and reaches its end; this row measure does not assert function entry |

Shadow-only tags have no suite-replaced credit. To put both profiles under one tag, run
`python tools/suite_coverage.py <tag> --combine <shadows-tag> <replace-tag>`; this snapshots the source batches under
the new tag, with combined summary/per-key tables. `sources.json` names only these bundled subdirectories.

`python tools/suite.py run --set all --profile shadows --reach --slots 12 --tag <tag> --dll <current-dll>` collects
the census through the shared queue. A DLL without the reach tracer is rejected before submission. Runs write only
slot copies and ignored artifacts, never the Documents region saves. `python tools/suite_coverage.py <tag> --select`
prints reach per scenario and set, saves `reach_scenarios.tsv`, `reach_sets.tsv` and `tier1-selection.json`, and uses
deterministic greedy byte-weighted set cover followed by redundant-row removal. The target is 98% of observed
implemented reach; candidates must finish cleanly, and conservative packing of measured run walls must fit 900
seconds on six slots. The heuristic is not a proof of global minimum set size. Missing traces remain explicitly
unmeasured. Saturation dates are candidates derived from first-hit windows; rerun a shortened scenario to measure it.
Selection is ineligible for tier1 while any current suite row or any scenario reach file is missing, even when the
98% target of the observed subset is met.
Mark the chosen rows `tier1` in `suite.tsv` only after measuring them. A marked tier runs directly, without local
calibration files. Its manifest records the actual batch wall (including queue waits), slot limit and completeness.
An interrupted census can continue with `python tools/suite.py resume <tag> --slots 12`; completed or claimed labels
are harvested rather than submitted again. Optional `--seconds` and `--timeout` bounds affect only unclaimed requests.

The regress `.out` read is `--regress`, else the PASS run of the newest lane merged into HEAD (first-parent
`Merge <tag>:` subjects), so the default depends on the commit, not on file times.

### Shadow evidence outside the regress run: `tools/coverage-shadows.tsv`

The house regress is the primary source of `verified`, but its shadows run exercises only the keys its list sets.
A shadow that a lane run alone has measured (a new glue lane's shadow before a `regresslists*` lane adds it, or
one kept out of the list on purpose) is recorded in `tools/coverage-shadows.tsv`: one row per compared set, with
its clean and total counts, the label the run uses for that set, and the file that records the run.

A row counts only when its `source` file literally contains `<label> <clean>/<total>`. The counts are therefore
checked against the run that measured them and never taken on trust from the ledger; a row that fails its check
(missing source, counts absent from it) voids its whole shadow key and is reported as a warning. `verified` is
credited only when every compared set of a key is exact, so a shadow with one differing set stays unverified and
the report shows its real counts (`fire 211/263, riot 2/354 (lane run, not exact)`) instead of implying the
subsystem is proven. The regress path still wins whenever the regress shadows list sets the key.

## Running it

```
python tools/coverage.py                               # writes docs/coverage-report.md
python tools/coverage.py --regress build-runs/regress-<tag>.out
python tools/coverage.py --owners build-runs/coverage-owners.tsv   # per-function audit TSV (address, bytes, owner, rule, subsystem); do not commit
python tools/coverage_treemap.py                       # writes docs/coverage-map.svg from the same attribution
```

About 6 seconds, deterministic (two runs write a byte-identical report; well under a minute). The report contains counts, class names, address ranges and statuses only, never
code. `coverage_treemap.py` reads `coverage.py` as a module and calls its `map_statuses()` for the four statuses,
so the two cannot drift: it colours the same owners by the furthest status their map row has reached, and its
header shares are summed on each status's own flag rather than on that colour, since a row can be implemented
without a spec or replaced without a module.

## Moving the numbers honestly

- A new spec, core, shadow or replace key: edit the subsystem's row in `coverage-map.tsv`. For a shadow the house
  regress does not run, add its measured counts to `tools/coverage-shadows.tsv` as well, each row citing the run
  that records them: the tool refuses a row whose source does not contain the counts it claims, so the ledger
  cannot invent a proof. Do not record a shadow at exact counts the run did not measure.
- Unattributed or shared bytes: name functions (a manual file in `re/symbols/` with evidence, then
  `python tools/symport/merge_names.py` regenerates `re/names_windows.tsv` and `re/labels_windows.tsv`), add vtables to
  `docs/abi`, a library range with its evidence to `coverage-libs.tsv`, or a game-code owner range with its evidence
  to `coverage-ranges.tsv`. Never add rows or ranges just to move a number; every range needs evidence in its row.
  Example: `0x008310B0-0x00864B60` (793 functions, 203,440 bytes, unattributed until 2026-10-16) is the software span
  renderer of `cGZDrawContextRenderToBuffer`. Its vtables and the five span renderer classes are named in
  `re/symbols/gzdraw-softrender.tsv`, and each class's `RenderBlt` indexes a 14 x 12 dispatch table of template blit
  and texel-conversion instantiations (each `RenderBlt` refers to 168 functions, so it is a hub and reaches nothing by
  the reach rule). Rather than one owner range for the whole block, it is five ranges, cut at the link-order anchor of
  each class's `RenderSolid` (`cSpanRender1555` 008310B0, `565` 0083DE50, `4444` 00847CE0, `8888` 00854E10, `8`
  0085E640, the last ending where the EA sound library begins), so each instantiation is owned by the class whose
  colour format it converts: 172 / 145 / 172 / 174 / 130 functions and 50,817 / 39,051 / 51,809 / 37,218 / 24,545
  bytes. The five map rows sit *before* the `nGZDrawSoftRender*` catch-all (first match wins) and each gets its own
  subsystem line, so the report shows the split instead of one blob. The 25 vtable methods already carry a name and
  reach their class by rule 2; the 768 unnamed instantiations take `owner-range`.

### Proven non-code references

`tools/coverage-noncode-refs.tsv` lists exact exported data-reference pairs proved to be data, with a public evidence citation. These pairs cannot propagate an owner as a function-pointer table. Every other edge, including a real call to the same target, remains eligible. Malformed, duplicate or stale records fail the measurement. The input is hashed in the report. This changes attribution only: no function or byte is removed from the denominator. The current `arg` string correction leaves the 42-byte export artifact uncredited and attributes exactly the 50 real ScriptServer bodies (7658 bytes).
