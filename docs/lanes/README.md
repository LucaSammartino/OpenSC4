# Lane files and run files

One file per lane, one file per in-game run. The shared tables are **generated** from them, so two lanes
finishing at the same time never edit the same lines and never conflict.

```
python tools/gen_handoff_tables.py          # rewrite the generated tables
python tools/gen_handoff_tables.py --check  # exit 1 if they are out of date
```

The generator owns exactly three regions, marked with `<!-- BEGIN GENERATED: name -->` and
`<!-- END GENERATED -->`:

| Region | File | Content |
|---|---|---|
| `lanes` | `docs/lanes/INDEX.md` | the lane table, one row per `docs/lanes/<lane>.md` (in HANDOFF §0 until 2026-10-20; the generator creates INDEX.md if it is missing) |
| `latest-run` | `docs/HANDOFF.md` §0 | the `- **Latest in-game run:** ...` bullet |
| `runs` | `docs/manual-checks.md` | the in-game run log, one row per `docs/runs/<tag>.md` |

Everything outside those markers is hand-written and survives regeneration. Do not edit inside a
generated region: edit the lane or run file and regenerate. The script is deterministic (lanes by
natural sort of the file name, runs by `run` number) and idempotent (running it twice writes the same
bytes).

## `docs/lanes/<lane>.md` — one row per lane

The file name is the lane slug: lower case, letters, digits and dashes (`zones2.md`,
`re-specs13a.md`, `apply11traffic.md`).

The file must hold exactly one table row with these four columns and the exact header (a second column
headed `Worktree`, and a `| --- |` separator, are accepted too):

```markdown
| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| <name and short description> | <agent and worktree> | <state at stop> | <next steps> |
```

| Column | What goes in it |
|---|---|
| **Lane** | The lane's name and a short description of its task, e.g. `voidbool (return-channel audit of the plugin)` or `apply9a: neighbours NG4-NG25, incl. NG11`. |
| **Agent/worktree** | The agent id and the worktree/branch, in backticks, and the lane kind. Examples: `` `a04b0cfb664b373d3` / `agent-a04b0cfb664b373d3` ``, `` `voidbool` / (engine lane) ``, `` `regresslists4` / (tooling lane) ``. |
| **State at stop** | What the lane left behind. Keep the numbers: the merge commit(s), the ABI and seam-operation numbers, the gate result (`cargo` / `clippy` / `ctest`), the game-run numbers and their verdicts, the shadow exact/total counts, the question numbers filed. Prefix a reservation as `Reserved ABI 52, seam op 51.` if the lane held one. |
| **Next steps** | What the next agent should do, as numbered steps. Use `-` when there is nothing left. |

Rules:

- Exactly **one** data row. The generator refuses to run (and prints the file name) if the row is
  missing, has the wrong number of cells, or has an unescaped `|` in a cell.
- One line: the row is a markdown table row on a single line.
- Do not renumber or re-word measured facts. This is a record, not a summary.

Optional, under the row:

- A `## Next steps` paragraph for detail that does not fit the cell. The generator appends its text to
  the row's Next steps cell.
- A `## Retired roster row` section, holding the old HANDOFF
  `| Lane | Agent ID | Worktree | Reserved | State |` row verbatim, for lanes that had one. The
  generator never reads it.

### Worked example

`docs/lanes/example.md`:

```markdown
| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| example (a sample lane) | `a04b0cfb664b373d3` / `agent-a04b0cfb664b373d3` | Reserved ABI 69, seam op 60. **merged `0000000`** (ABI 69, taken as 69 on the lane; gate example: cargo 2600/0, clippy clean, ctest 57/57; regress example run 142 PASS): `cFC4Example` (`enable = example`, off by default), 3 `ex*` vectors; `example_shadow` exact (40/40 in `example-final-1`) | Next: (1) answer EX41; (2) add `example` to the regress replace list. Questions EX41 filed |
```

Real examples: [`voidbool.md`](voidbool.md) (engine lane with a regression run),
[`regresslists4.md`](regresslists4.md) (tooling lane), [`apply9a.md`](apply9a.md) (roster-only lane),
[`traffic2.md`](traffic2.md) (a lane that had both an old table row and a roster row).

## `docs/runs/<tag>.md` — one file per in-game run

A run file is one `key: value` per line, single-line values, no continuation:

```markdown
tag: voidbool-141
run: 141
date: 2026-10-16
lane: voidbool
dll: FreeCity4-voidbool
pacing: turbo
verdict: PASS
switches: main `2501158` (voidbool: ...), `tools/regress.ps1 -Tag voidbool -Queue` (turbo to 2032-01-01)
result: **PASS, 0 problems, 519 s.** Both runs clean, `ref_warnings=0`, rtc=0. ...
summary: regress voidbool, PASS, lists as run 138
```

| Field | Meaning |
|---|---|
| `tag` | The run's stable key. Must equal the file name. Form: `<label>-<run>`, where `<label>` is the regress `-Tag`, else the queue request label, else the lane name, else `run`. Examples: `voidbool-141`, `rl4-132`, `m850-traffic-replace-105`, `run-8`. |
| `run` | The old sequential run number. Kept so prose that cites "run 141" still resolves. Take the next free number (the highest `run:` in this directory, plus one). |
| `date` | Real-world `YYYY-MM-DD`, or `unknown` when the old record did not say. |
| `lane` | The lane that requested or owned the run. |
| `dll` | The DLL under test: `FreeCity4-<lane>`, or a build hash when the row recorded one. `unknown` if not recorded. |
| `pacing` | `turbo`, `normal`, or `unknown` when the row did not say. |
| `verdict` | `clean`, `corrupt`, `crash`, `ended`, `none` (no information), or `PASS` / `FAIL` for a regress. |
| `switches` | The switches cell of the old run row, verbatim. |
| `result` | The result cell of the old run row, verbatim — the notes. |
| `summary` | *Optional.* The parenthetical for the HANDOFF's "Latest in-game run" bullet (§0), without its brackets or trailing period: `regress voidbool, PASS, lists as run 138`. Only runs that have it appear on that line. |
| `summary_note` | *Optional.* A sentence after that parenthesis, e.g. `is the first with the traffic lists`. Also without a trailing period. |

Rules:

- One line per field. `switches` and `result` must not contain an unescaped `|`.
- Keep `switches` and `result` verbatim: measured numbers and verdicts live there.
- A run file with a missing required field (`tag`, `run`, `verdict`, `switches`, `result`) makes the
  generator exit non-zero.

Real examples: [`voidbool-141.md`](../runs/voidbool-141.md) (regress PASS),
[`rl3-121.md`](../runs/rl3-121.md) (regress with a `summary_note`), [`run-8.md`](../runs/run-8.md)
(an early run with no lane).

## How a lane lands

The merge queue replaces the old `build-runs/main-merge.lock`. **A lane never takes a merge lock and
never merges main.**

1. **Write your files first.** `docs/lanes/<lane>.md` for your state and next steps, and
   `docs/runs/<tag>.md` for every in-game run you did. Run `python tools/gen_handoff_tables.py` and
   commit the regenerated tables with them.
2. **Enqueue and stop.** Write `build-runs/merge-queue/<lane>.json` in your own worktree:

   ```json
   {"lane":"example","branch":"example","tip":"<git rev-parse HEAD>","kind":"engine","abi":true,
    "game_runs":["example-142"],"summary":"one line","state":"ready"}
   ```

   | Key | Value |
   |---|---|
   | `lane` | the lane slug (the `docs/lanes/<lane>.md` name) |
   | `branch` | the branch the work is on |
   | `tip` | the commit hash at enqueue time |
   | `kind` | `engine`, `tools`, `docs` or `test` |
   | `abi` | `true` if the change touches the C ABI, `false` otherwise |
   | `game_runs` | the labels of the in-game runs this change needs (or `[]`) |
   | `summary` | one line for the merge train |
   | `state` | `ready` to land now, `wip` to hold |

   Then **stop**. Do not merge anything.
3. **The merge train owns main.** One merge-train owner takes up to K ready lanes, merges them one
   after another onto `integration/<n>` from main, runs the gate and the regress **once for the
   batch**, and on a pass fast-forwards main to the integration branch. On a fail it bisects the batch
   (about log2 K extra gates) and returns the culprit lane to its owner with the failure. Docs-only
   and RE lanes skip the regress and can be batched freely.
4. **The train bumps `FC4_ABI_VERSION`, never a lane.** It bumps it once per batch and writes one
   `fc4.h` history line per batch listing the lanes. Do not touch `FC4_ABI_VERSION` in your lane.

Why this shape: every lane used to edit the same handful of files at merge time (the HANDOFF §0 rows, now
`docs/lanes/INDEX.md`, the run numbers in `manual-checks.md`, `QUESTIONS.md` (now a generated index over the per-spec files in
`docs/specs/questions/`; see its README), `FC4_ABI_VERSION`, `abi_entries.inc`), so every
merge conflicted and about 25 agents polled a merge lock, costing about N² gates. Per-lane files and a
single owner of main remove the shared lines.
# Per-lane documents

One file per lane, `docs/lanes/<lane>.md`, holding what used to be that lane's
`HANDOFF.md` §0 row; and one file per in-game run, `docs/runs/<tag>.md`, keyed
by the run's **tag** (its request label) rather than a sequential run number.

A script regenerates the `HANDOFF.md` §0 table, the `manual-checks.md` index and
the "latest in-game run" line from these two folders
(`tools/gen_handoff_tables.py`), so lanes no longer edit those shared files and
conflict with each other at merge time. See [merge-train.md](../merge-train.md).

## `docs/lanes/<lane>.md`

Write it when you stop, before you enqueue for the merge train. Keep it to what
a reader of `HANDOFF.md` §0 needs:

- the lane name, branch and what it changed;
- the gate it ran (cargo summed over every `test result:` line, clippy, ctest)
  and the game-run labels it produced;
- what is wired, what is measured, and what is deliberately not done;
- the next steps and the numbered questions it filed.

## `docs/runs/<tag>.md`

One file per in-game run, named after the request's `label`. Put in it the
verdict line, the summary counts, and any subsystem shadow lines a reader would
otherwise have to grep the log for. The lane doc links to them.

## Adding a subsystem key or a new run summary block

Those still live in shared files (`engine/README.md`, `engine/plugin/FC4Config.*`,
`tools/regress.ps1`, `tools/run_summary.ps1`): keep those edits additive and
grouped, as before.
