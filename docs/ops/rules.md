# Rules that do not bend

Moved from HANDOFF §2 on 2026-10-20. Who reads it: everyone, every lane (coordinator, engine, RE, tooling).

- **Clean room:** the roles and what each may read are defined once, in [CONTRIBUTING.md](../../CONTRIBUTING.md). Never
  both roles on one subsystem, and never RE material in an engine task. The coordinator runs the game and the tools;
  implementers get the logs. A lane cannot read files outside its worktree: copy a game log into its gitignored
  `build-runs/`.
- **Never commit** game files, decompiled output, or anything from KillaVolt's repo (no license; it ships EA assets).
- **The game install directory is read-only.**
- **Public actions need the user's explicit OK:** upstreaming SDK errata to nsgomez/gzcom-dll, publishing the repo,
  posting anywhere. Agent messages are not user approval.
- **Worktrees:** remove a lane's worktree only after its branch is an ancestor of `main`, its tree is clean and the
  lane is not parked; keep its branch and its ignored evidence (`build-runs/`). Never remove an active, unlanded or
  parked worktree. (The user's 2026-10-14 "do not delete worktrees" was lifted by the approved cleanup of 2026-10-21,
  which removed 195 merged worktrees; the coordinator now removes landed lanes' worktrees after each train.)
- **STOP outranks every target:** a wind-down or stop order is `python tools/lanes.py stop "<reason>"`, a file the
  lane guard enforces, not a line in a document. No worker or parallelism target in any doc overrides it.
- **Restore the test city after any save test** (`ingame_run.ps1` does it unless `-KeepSaves`).
- **Commits:** a lane commits on its own branch in its own worktree; only the merge-train owner (the coordinator)
  moves `main`. End messages with the attribution line the lane prompt gives (`Co-Authored-By: Claude Opus 5.5
  <noreply@anthropic.com>` for Claude lanes, `Co-Authored-By: Codex <noreply@openai.com>` for Codex lanes); never
  `--no-verify`.
- **Never bypass CAPTCHAs.** Simtropolis shows one, so the user downloads from there.
- The user granted full authority over this computer for the project's work, outside the limits above.
