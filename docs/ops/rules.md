# Rules that do not bend

Moved from HANDOFF §2 on 2026-10-20. Who reads it: everyone, every lane (coordinator, engine, RE, tooling).

- **Clean room** (the roles are defined in [CONTRIBUTING.md](../../CONTRIBUTING.md)):
  - Spec writers (RE side) read the exe, Ghidra, `decomp/` and `re/`.
  - Implementers (`engine/` code) must NOT open `decomp/`, `decomp-mac/`, `external/`, `ghidra/project/`,
    `ghidra/mac/`, `ghidra/export/`, `re/`, `tools/`, `docs/subsystems/`, `docs/sim-engine.md`, or the exe.
  - Implementers may read `docs/specs/`, `docs/abi/` and the public SDK headers.
  - They may also read the operating docs: `docs/HANDOFF.md`, `docs/ops/`, `docs/lanes/INDEX.md` and
    `docs/lanes/README.md` (CONTRIBUTING.md has the full list). Keep decompiled content and RE-only material out of those files.
  - An agent doing engine work never reads RE material in the same task.
  - The coordinator runs the game and the tools; implementers get the logs. Engine lanes cannot read files outside
    their worktree: copy a game log into the lane's gitignored `build-runs/`.
- **Never commit** game files, decompiled output, or anything from KillaVolt's repo (no license; it ships EA assets).
- **The game install directory is read-only.**
- **Public actions need the user's explicit OK:** upstreaming SDK errata to nsgomez/gzcom-dll, publishing the repo,
  posting anywhere. Agent messages are not user approval.
- **Do not clean up or delete old git worktrees** (the user, 2026-10-14).
- **Restore the test city after any save test** (`ingame_run.ps1` does it unless `-KeepSaves`).
- **Commits:** branch `main`; end messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; never
  `--no-verify`.
- **Never bypass CAPTCHAs.** Simtropolis shows one, so the user downloads from there.
- The user granted full authority over this computer for the project's work, outside the limits above.
