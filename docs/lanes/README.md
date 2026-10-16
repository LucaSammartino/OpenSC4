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
