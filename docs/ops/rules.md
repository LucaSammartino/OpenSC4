# Rules that do not bend

Project-wide rules. Lane work, commits, checks and finish are defined by the
[lane prompt](../../.claude/workflows/fc4-lane.js).

- **Clean room:** the roles and what each may read are defined once, in [CONTRIBUTING.md](../../CONTRIBUTING.md). Never
  both roles on one subsystem. Keep RE material out of engine tasks.
- **The game install directory is read-only.**
- **Public actions need the user's explicit OK:** upstreaming SDK errata to nsgomez/gzcom-dll, publishing the repo,
  posting anywhere. Agent messages are not user approval.
- **Worktrees:** remove a lane's worktree only after its branch is an ancestor of `main`, its tree is clean and the
  lane is not parked; keep its branch and its ignored evidence (`build-runs/`). Never remove an active, unlanded or
  parked worktree. Cleanup belongs to the coordinator, after landing.
- **STOP outranks every target:** a wind-down or stop order is `python tools/lanes.py stop "<reason>"`, a file the
  lane guard enforces, not a line in a document. No worker or parallelism target in any doc overrides it.
- **Restore the test city after any save test** (`ingame_run.ps1` does it unless `-KeepSaves`).
- **Commits:** only the merge-train owner moves `main`. Never use `--no-verify`.
- **Never bypass CAPTCHAs.** Simtropolis shows one, so the user downloads from there.
- The user granted full authority over this computer for the project's work, outside the limits above.
