# Lane files and run files

Everything a lane writes for the record: its lane file, one file per in-game run, and its merge-queue entry. The
shared tables are **generated** from the lane and run files, so two lanes finishing at the same time never edit the
same lines. The merge train regenerates them after each batch; a lane does not need to.

```
python tools/gen_handoff_tables.py          # rewrite the generated tables (the train runs this)
python tools/gen_handoff_tables.py --check  # exit 1 if they are out of date
```

Engine lanes do not run scripts from `tools/` (clean room, [CONTRIBUTING.md](../../CONTRIBUTING.md)): get the row
format right by the rules below.

The generator owns exactly three regions, marked with `<!-- BEGIN GENERATED: name -->` and
`<!-- END GENERATED -->`:

| Region | File | Content |
|---|---|---|
| `lanes` | `docs/lanes/INDEX.md` | the lane table, one row per `docs/lanes/<lane>.md` (in HANDOFF §0 until 2026-10-20; the generator creates INDEX.md if it is missing) |
| `latest-run` | `docs/HANDOFF.md` §0 | the `- **Latest in-game run:** ...` bullet |
| `runs` | `docs/manual-checks.md` | the in-game run log, one row per `docs/runs/<tag>.md` |

Everything outside those markers is hand-written and survives regeneration. Do not edit inside a
generated region: edit the lane or run file instead. The script is deterministic (lanes by
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
- While the lane works: a `## Handoff` section (at most 25 lines: goal, done with commit hashes, next,
  facts), under a row whose State at stop is `in progress`. It is the lane's working state: the fc4-lane
  workflow restarts a lane in a fresh agent from it (docs/ops/coordinating.md "Lane context"). The lane
  replaces it with its report when it finishes. The generator never reads it.
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

Also put in a run's `result` the summary counts and any subsystem shadow lines a reader would otherwise have to grep
the log for; the lane file links to its runs.

Real examples: [`voidbool-141.md`](../runs/voidbool-141.md) (regress PASS),
[`rl3-121.md`](../runs/rl3-121.md) (regress with a `summary_note`), [`run-8.md`](../runs/run-8.md)
(an early run with no lane).

## How a lane lands

A lane never takes a merge lock, never merges main and never bumps `FC4_ABI_VERSION`.

1. **Write your files.** `docs/lanes/<lane>.md`: the row, then below it your full report in place of the
   `## Handoff` (what changed; the gate, with cargo summed over every `test result:` line, clippy and ctest; the
   game-run labels; what is measured and what is deliberately not done; questions filed; next steps). A
   `docs/runs/<tag>.md` for every in-game run you did. Commit them.
2. **Enqueue and stop.** Write `build-runs/merge-queue/<lane>.json` in your own worktree (`build-runs/` is
   gitignored; the train scans every `.claude/worktrees/*/build-runs/merge-queue/` and the main checkout's):

   ```json
   {"lane":"example","branch":"example","tip":"<git rev-parse HEAD>","kind":"engine","abi":true,
    "game_runs":["example-142"],"summary":"one line","state":"ready"}
   ```

   | Key | Value |
   |---|---|
   | `lane` | the lane slug (the `docs/lanes/<lane>.md` name) |
   | `branch` | the branch the work is on |
   | `tip` | the commit hash at enqueue time (`merge-train.py status` reports an entry whose branch moved past it as not ready) |
   | `kind` | `engine`, `tools`, `docs` or `test`; `docs` batches go first and skip the gate when `engine/`, `tools/`, `tests/` and `.claude/lane-bin/` are unchanged |
   | `abi` | `true` if the change adds or removes a C ABI export, `false` otherwise |
   | `game_runs` | the labels of the in-game runs this change needs (or `[]`) |
   | `summary` | one line for the merge commit |
   | `state` | `ready` to land now, `wip` to hold |
   | `touches` | *optional*: the subsystem keys the work changes, as given to `lanes.py check --touches` |

   Then **stop**. What the train does next (batches, one gate and regress per batch, bisecting a red batch, the ABI
   bump) is in [docs/merge-train.md](../merge-train.md); a notice lands in `build-runs/merge-notices/<lane>.md`
   when you are on `main`.

Shared files that still take every lane's edits (`engine/README.md`, `engine/plugin/FC4Config.*`,
`FC4Subsystems.cpp`, `tools/regress.ps1`, `tools/run_summary.ps1`): keep those edits additive, grouped and at the end
of their lists.
