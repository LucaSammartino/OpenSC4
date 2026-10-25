# Lane files and run files

Record formats for the [lane prompt's](../../.claude/workflows/fc4-lane.js) finish procedure.
The merge train regenerates shared tables; lanes edit only their own records, never generated regions.

```
python tools/gen_handoff_tables.py          # rewrite the generated tables (the train runs this)
python tools/gen_handoff_tables.py --check  # exit 1 if they are out of date
```

Engine lanes do not run these tools ([clean room](../../CONTRIBUTING.md)). Generated regions use
`<!-- BEGIN GENERATED: name -->` and `<!-- END GENERATED -->`:

| Region | File | Content |
|---|---|---|
| `lanes` | `docs/lanes/INDEX.md` | one row per lane file; created if missing |
| `latest-run` | `docs/HANDOFF.md` §0 | the `- **Latest in-game run:** ...` bullet |
| `runs` | `docs/manual-checks.md` | the in-game run log, one row per `docs/runs/<tag>.md` |

Outside text survives regeneration. Lanes sort naturally by filename; runs sort by `run` number.

## `docs/lanes/<lane>.md` — one row per lane

The filename is the lane slug: lowercase letters, digits and dashes.

Exactly one data row, four cells, on one line. Header (`Worktree` is also accepted for column 2):

```markdown
| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| <name and short description> | <agent and worktree> | <state at stop> | <next steps> |
```

| Column | What goes in it |
|---|---|
| **Lane** | slug and short task description |
| **Agent/worktree** | agent/worktree or branch in backticks, and lane kind |
| **State at stop** | commits, ABI/seam reservations if any, gate counts, game labels/verdicts, shadow exact/total, question IDs; keep measured facts exact |
| **Next steps** | numbered actions, or `-` |

Escape `|` inside cells. Invalid rows stop generation. Write the lane file once at Finish, with at most
25 report lines below the row; an interrupted round follows the prompt's handoff instructions.
The generator appends an optional `## Next steps` section to the last cell and ignores `## Handoff`
and `## Retired roster row` sections.

## `docs/runs/<tag>.md` — one file per in-game run

A run file is one `key: value` per line, no continuations. Format example:

```markdown
tag: example-1
run: 1
date: YYYY-MM-DD
lane: example
dll: FreeCity4-example
pacing: turbo
verdict: PASS
switches: <build and settings>
result: <counts, shadows and verdict>
summary: <optional latest-run description>
```

| Field | Meaning |
|---|---|
| `tag` | Filename stem: `<label>-<run>`; label is regress `-Tag`, else request label, lane name, or `run`. |
| `run` | Next free sequential number: highest `run:` plus one. Preserve existing numbers. |
| `date` | Real-world `YYYY-MM-DD`, or `unknown`. |
| `lane` | The lane that requested or owned the run. |
| `dll` | `FreeCity4-<lane>`, build hash, or `unknown`. |
| `pacing` | `turbo`, `normal`, or `unknown`. |
| `verdict` | `clean`, `corrupt`, `crash`, `ended`, `none` (no information), or `PASS` / `FAIL` for a regress. |
| `switches` | Exact build/settings and command. |
| `result` | Exact measured counts, verdicts and subsystem shadow lines. |
| `summary` | Optional latest-run bullet parenthetical, without brackets or trailing period. Only runs with it appear there. |
| `summary_note` | Optional sentence after that parenthetical, without trailing period. |

Required: `tag`, `run`, `verdict`, `switches`, `result`; omission fails generation.
Escape `|` in `switches` and `result`. Preserve existing measured text verbatim. Link your runs in the lane file.

## How a lane lands

After the prompt's Finish commits, write `build-runs/merge-queue/<lane>.json` in your worktree and stop.
The train scans every `.claude/worktrees/*/build-runs/merge-queue/` and the main checkout's:

   ```json
   {"lane":"example","branch":"example","tip":"<git rev-parse HEAD>","kind":"engine","abi":true,
    "game_runs":["example-142"],"summary":"one line","state":"ready"}
   ```

   | Key | Value |
   |---|---|
   | `lane` | the lane slug (the `docs/lanes/<lane>.md` name) |
   | `branch` | the branch the work is on |
   | `tip` | full 40-character HEAD at enqueue time; a branch that moved past it is not ready |
   | `kind` | `engine`, `tools`, `docs` or `test`; `docs` batches go first and skip the gate when `engine/`, `tools/`, `tests/` and `.claude/lane-bin/` are unchanged |
   | `abi` | `true` if the change adds or removes a C ABI export, `false` otherwise |
   | `game_runs` | the labels of the in-game runs this change needs (or `[]`) |
   | `summary` | one line for the merge commit |
   | `state` | `ready` to land now, `wip` to hold |
   | `touches` | *optional*: the subsystem keys the work changes, as given to `lanes.py check --touches` |

The [merge train](../merge-train.md) writes `build-runs/merge-notices/<lane>.md` after landing.

Shared files that still take every lane's edits (`engine/README.md`, `engine/plugin/FC4Config.*`,
`FC4Subsystems.cpp`, `tools/regress.ps1`, `tools/run_summary.ps1`): keep those edits additive, grouped and at the end
of their lists.
