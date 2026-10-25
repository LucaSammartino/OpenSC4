# The merge train

The coordinator owns `main`; [`tools/merge-train.py`](../tools/merge-train.py) lands queued batches.
Operator commands, conflict handling and gate recovery belong to
[coordinating.md](ops/coordinating.md#landing-work-the-merge-train).

Lanes follow the [prompt](../.claude/workflows/fc4-lane.js) and [queue contract](lanes/README.md#how-a-lane-lands).

## What the train does

1. **Batch (up to K lanes), oldest first.** Docs-only lanes go first: they never need a regress.
2. **Merge them one after another onto `integration/<n>`, cut from `main`.** Conflicts are resolved once for the
   whole batch, not per lane.
3. **Gate and regress once for the batch** (skipped when `engine/`, `tools/`, `tests/` and `.claude/lane-bin/` are
   all unchanged). Changes to `docs/abi/interfaces.tsv` also require a gate: it generates a conformance test.
   Gate records identify the exact head, clean tree, parsed step logs and DLL hash.
4. **Pass:** `land` rechecks that record and fast-forwards `main`. It refuses if build files changed on `main`
   since the batch was cut. There is no manual gate override.
5. **Fail:** bisect by gating halves sequentially in the integration worktree. Send the culprit back with `fail`;
   merge and gate the remaining lanes as a new batch.

The long-lived integration worktree is `.claude/worktrees/train-integration` (branch `train-integration`); it is
re-pointed at `integration/<n>` for every batch and keeps its cargo/CMake caches.

## ABI version

The train calls `tools/bump_abi.py --batch <id> --lanes a,b,c` once per ABI batch (built-in fallback if absent).
It records the bump in `build-runs/abi-batches.json` and appends a `fc4.h` history line naming the lanes.

## State

`build-runs/merge-train/state.json` (gitignored): the batch sequence, the current batch, and the landed history.
Notices are `build-runs/merge-notices/<lane>.md` and `build-runs/merge-notices/train<n>.md`. Gate output is
`build-runs/gate-train<n>.out` (summary) with the record `build-runs/gate-train<n>.json` and the step logs in
`build-runs/gate-logs/train<n>/`, regress output `build-runs/regress-train<n>.out` (both UTF-8 — the PowerShell
`*>` files elsewhere in `build-runs/` are UTF-16 and need `tr -d '\0'`).
