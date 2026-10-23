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
  stated rules. The legacy baseline has class granularity; the reviewed ledger below adds exact entries without
  promoting their owners.

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
gives the subsystem, its area (simulation, framework, renderer, ui, audio, region, other) and four things the
statuses are computed from: spec files, core module paths, shadow ini key, replace ini key. The report warns about
missing spec or core files, unknown keys and rows that match nothing; it also lists the biggest owners only a
catch-all row matches, the candidates for a new row.

## Legacy statuses (per map row)

| Status | Rule |
|---|---|
| specified | the row names at least one spec and every named `docs/specs/<file>` exists; a spec counts only for the classes that are its subject |
| implemented | the row names at least one core path and every `engine/<path>` exists |
| verified | every compared set of the row's shadow key is exact. Two sources, in this order: (1) the key is in the shadows list of `tools/regress.ps1` and every line of that shadow in the regress `.out` read has all mismatch counts 0; (2) else `tools/coverage-shadows.tsv` has the key and every row it records is exact |
| replaced | the replace key is in `$Replace` of `tools/regress.ps1` (or is on by default in `FC4Subsystems.cpp`) and the regress replace run was clean |

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

## Exact-function staged evidence: `tools/coverage-functions.json`

This is a separate, conservative capability for an otherwise partial owner. The initial tracked ledger is empty;
the [Registry pilot review](coverage-functions/registry-review.md) withholds all three candidate entries. It does
not amend the completed Directory/HTML owner qualification. Neither this format nor a hash proves semantic
correspondence. Reviewed, published evidence is required before adding a record; never fill records from generic
getters, mock host outcomes, a test count, or a convenient supported subset of a known valid normal domain.

`coverage.py.function_statuses()` is the shared resolver for report headline, area, subsystem, affected-owner
breakdown, audit TSV and treemap. For each actual entry, each stage is the union of its accepted legacy row flag
and its qualified entry flag. A function and its actual body bytes count once per stage; implemented and specified
are independent. No stage implies another. Verified/replaced still come exclusively from the legacy policies;
the new ledger cannot claim them. Shared, library and unowned functions receive no ledger credit. Attribution,
names, body boundaries, exclusions, owner ranges and denominator do not change.

The report exposes the owner-only baseline and exact additions separately. Newly covered owners remain **partial**,
even if the ledger eventually covers every attributed entry. Each affected-owner/subsystem cell gives exact
completed/total functions and bytes. Treemap aggregation splits the owner's entries by resolved stage; the colour
of one completed method cannot colour its siblings. Header totals use independent flags rather than colour.
Audit TSV retains its five original columns and appends four resolved flags and four `owner_` baseline flags (0/1).

### Version 1 schema (all keys required; unknown keys rejected)

The UTF-8 JSON root is exactly `{"version": 1, "claims": []}`. Each claim is an object with these fields:

| Field | Required value |
|---|---|
| `address` | eight uppercase hexadecimal digits, exactly an existing native entry, never an interior/caller/context address |
| `owner` | exact expected current Attribution owner, with no owner reassignment |
| `stages` | nonempty unique list drawn only from `specified`, `implemented`; order must match qualification |
| `artifact` | exact current object containing `exe_sha256`, `functions_sha256`, `details_sha256`, `attribution_sha256` |
| `evidence` | roles `contract`, `qualification`; implemented additionally requires `production`, `assertions`, `validation` |

`functions_sha256` and `details_sha256` hash the unchanged export files. `attribution_sha256` is SHA-256 of the
ASCII full normalized digests joined by LF (no final LF), in this fixed order: export `calls.tsv`,
`data-references.tsv`, `instructions.asm`, then repository `re/names_windows.tsv`, `docs/abi/vtables.tsv`,
`docs/abi/interfaces.tsv`, `tools/coverage-libs.tsv`, `tools/coverage-ranges.tsv`, `tools/coverage-noncode-refs.tsv`.
These are identity inputs, not permission to research original bodies. Bytes are never input in a claim.
Current owner, library/shared status, folded-name rule, vtable ambiguity and overlapping bodies are checked
against unchanged Attribution. Folded entries are conservatively withheld, including same-owner folds.

Each evidence reference is exactly `{"path": "...", "sha256": "<64 lowercase hex>", "citation": "<path>#<section>"}`.
The path must be a tracked repository file, with forward slashes, no absolute/drive paths, traversal, symlink
escape or missing file. Contract paths are under `docs/specs/`; production under `engine/`; assertions under
`engine/` or `tools/tests/`; qualification and retained public validation under `docs/coverage-functions/`.
Roles use distinct files. The ledger itself is tracked under `tools/`. Missing, stale, duplicated or conflicting
records, duplicate JSON fields, unsupported stages and malformed citations **fail the measurement closed** before
writing a report, SVG or audit. Existing warnings and strict-input checks remain in force.

The qualification file is human-reviewable UTF-8 JSON with exactly these fields:

| Field | Required value |
|---|---|
| `address`, `owner`, `stages` | identical to the claim |
| `review` | object with nonempty `by`, `citation` and Boolean `approved: true`; cite the actual independent review |
| `normal_domain_complete` | Boolean `true`, only after reviewing every supported normal branch/effect |
| `decisions` | nonempty list of objects with `contract`, `production`, `assertion` correspondence text |
| `limitations` | explicit nonempty representation/native undefined-memory/reentry/other limitations, or `none` |
| `validation` | specified-only: null; implemented: the object below |

Each decision must cite contract text actually present in the bound public contract and, for implemented,
authoritative production symbol/text and independent assertion symbol/text actually present in their bound files.
For specified-only, production/assertion values must be null. Review must establish the whole function's
published supported decision/effect contract, including null/zero/full-width cases where defined. A known valid
unimplemented branch withholds the entire function; there is no 25/50/90 percent credit or safe-domain relabeling.
Presence checks constrain references but do not prove that assertions independently establish the correspondence.

Implemented `validation` has exactly `command`, `result`, `production_sha256`, `assertions_sha256`, `log_sha256`,
`provenance`. Command and provenance are nonempty; result must be `PASS`; all three digests must match the bound
current production/assertions/retained-validation files. The retained public validation must literally contain the
command, `result=PASS`, `production_sha256=<digest>` and `assertions_sha256=<digest>`. It must preserve actual
command/start/end/result, source stability and original log digests/provenance and exact limitations for human
review. A summary invented from a test count is inadmissible even if it passes mechanical checks. No engine
execution or new native proof is authorized by this accounting feature. Hash the qualification itself, too;
editing it or any cited source/spec/test/log without renewed review invalidates the record.

All new input hashes are full normalized SHA-256 values in report inputs and SVG metadata. Normalization reuses
`coverage.py.text_sha256` (CRLF to LF), so line-ending changes alone do not withdraw credit. Repeated runs on
identical inputs are deterministic; an empty ledger preserves every legacy count exactly.

## Running it

```
python tools/coverage.py                               # writes docs/coverage-report.md
python tools/coverage.py --regress build-runs/regress-<tag>.out
python tools/coverage.py --owners build-runs/coverage-owners.tsv   # per-function audit TSV (address, bytes, owner, rule, subsystem); do not commit
python tools/coverage_treemap.py --regress build-runs/regress-<tag>.out  # pin the same input as the report
```

About 6 seconds, deterministic (two runs write a byte-identical report; well under a minute). The report contains counts, class names, address ranges and statuses only, never
code. `coverage_treemap.py` reads `coverage.py` as a module and calls its shared `function_statuses()` after
`map_statuses()`, so the two use the same entry flags: it splits owners by each entry's furthest stage, and its
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
