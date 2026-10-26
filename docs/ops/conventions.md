# Working conventions

Environment details for lanes and the coordinator. The [lane prompt](../../.claude/workflows/fc4-lane.js)
owns cost rules, file editing, commit format and check cadence.

- **Harness design:** prefer off-by-default, logged plugin switches and scripted actions for automation.
- **Long commands:** use the longest supported tool-call wait (at least 60 s when available). Collect the same
  running operation to its terminal result; do not launch duplicate checks.
- **Run cargo from `engine/`**, never the worktree root: there `cargo fmt --check` prints help and exits 0.
- **rustfmt: check only the files you touched**, `python <MAIN>/.claude/lane-bin/qbuild.py --wt <WT> fmt <files>`;
  never mass-reformat. Clippy uses `-D warnings`.
- **Read results, not exit codes alone:** sum every `test result:` line from a captured file (never a `tail`); read
  ctest's verdict line (`0 tests failed` matches a grep for "failed").
- **Git:** before deleting any branch, prove nothing on it is unique. Worktrees made by an elevated session need
  `safe.directory = C:/Users/tlori/FreeCity4/.claude/worktrees/*` (in the global git config).
- **PowerShell 5.1** mangles `"` in native arguments: write commit messages to a file, UTF-8 without BOM,
  `git commit -F`. PowerShell variable names are case-insensitive (`$a` clobbers `$A`).
- **Build infra:** build through `qbuild.py` / `fc4verify.py`, never a bare cmake: they set a private
  `_MSPDBSRV_ENDPOINT_` (the shared 32-bit `mspdbsrv` runs out of address space with many lanes, LNK1318) and the
  sccache server per worktree ([coordinating.md](coordinating.md) "Build cache, lane checks and the gate").
