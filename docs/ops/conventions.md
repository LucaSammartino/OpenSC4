# Working conventions

Moved from HANDOFF §8 on 2026-10-20. Who reads it: everyone; engine and tooling lanes before their first
build or commit.

- **Bootstrap the harness with the engine** (the user, 2026-10-14): as more of the game is ours, use it to make the
  feedback loops short and automated (a turbo clock, plugin-side scripted actions instead of screen clicks, end
  dates instead of timeouts). Prefer a tooling switch in the plugin (off by default, logged) over UI automation.
- **Run cargo from `engine/`**, never the worktree root: there `cargo fmt --check` prints help and exits 0.
- **`fmt --check` has pre-existing diffs on main** (8 files in `capi/src` and `core/src/lot_developer/tuning.rs`).
  Compare against main's list; add none; do not mass-reformat. **Clippy is clean**; a new error is a regression.
- **Read results, not exit codes alone:** sum every `test result:` line from a captured file (never a `tail`); read
  ctest's verdict line (`0 tests failed` matches a grep for "failed").
- **Git:** prefer `git -C <path>` over `cd` (a stray `cd` once committed into another lane's worktree). Before deleting
  any branch, prove nothing on it is unique. Worktrees made by an elevated session need
  `safe.directory = C:/Users/tlori/FreeCity4/.claude/worktrees/*` (in the global git config).
- **PowerShell 5.1** mangles `"` in native arguments: write commit messages to a file, UTF-8 without BOM,
  `git commit -F`. PowerShell variable names are case-insensitive (`$a` clobbers `$A`).
- **`build-runs/` is gitignored:** gate logs, run logs, DLL copies and lane prompts live there; record the numbers in
  docs, not the files.
- **Build infra:** with many lanes building, the shared 32-bit `mspdbsrv` runs out of address space (LNK1318); the
  gate and lane scripts set a private `_MSPDBSRV_ENDPOINT_`.
- **Status reports for the user:** plain language, short, what changed and what is next.
- **Memory:** the coordinator's notes are in `C:\Users\tlori\.claude\projects\C--Users-tlori-FreeCity4\memory\`.
