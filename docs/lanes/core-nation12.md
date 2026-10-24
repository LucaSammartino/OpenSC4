| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation12 — nation NQ4 closure | Codex / core-nation12 | ready: NQ4 implemented; all/verify PASS | Coordinator land; portable owner closed |

## Report

cSC4Nation's portable core is now fully implemented against the settled nation.md
§§1–8 contract and its economy cross-reference to demand.md §11.2. No owner
sections remain open or Unsupported. The module README's Owner closure table
maps every section, including §§5, 5.1 and 5.2 conformance, to code and tests.
Native COM adaptation and game integration remain separate integration work.

Merged `re-nation-nq4` as `e8a8a38ca`. Question-index conflicts were resolved by
retaining the current unrelated mysims entries and updating only NQ4's counts.
Implementation commit: `4ab983b5b`; later commits record validation milestones.

### Changes

- Replaced the ASCII-only placeholder with `lower_directory`, accepting raw
  UTF-8 bytes and querying the host's active CRT narrow/wide lowercase mappings.
- Implemented the initial narrow-byte path, the DEL/high-byte switch to whole
  string conversion, failure-prefix truncation, four-byte-to-low-word truncation,
  one-word lowercase and legacy UTF-8 re-encoding. Failed maps retain the word;
  the transform performs no normalization or multi-character expansion.
- Init and Reset compare folded catalog names against registry bytes verbatim.
  Reset retains the first match. Downloads exclusion uses the same transform on
  both strings. Preference display-name comparison remains bytewise/case sensitive.
- Added all ten nation §5.2 vectors with explicit C/en-US fixtures, plus tests
  for DEL, incomplete/invalid prefixes, failed mapping and a locale change after
  registry loading. Mapping fixtures use no ambient test-runner locale.

### Measured validation

- Focused Rust tests: **25 passed**, zero failed (`nation`: 2; `nation_owner`: 23).
- Explicit owned-file formatting check: **PASS**, two files, zero diff lines.
- `qbuild all`, once on final engine source: **PASS**. Workspace totals summed
  over all 48 `test result:` lines: **7,224 passed, 0 failed, 0 ignored**.
  Clippy with `-D warnings`: **PASS**, zero diagnostics.
- All's native queue request `qbuild-20261024-140500-build-ctest`: **pass**;
  build **262/262 edges** (131.3 s), CTest **81/81** (188.2 s), total 324.0 s.
- Separately requested `qbuild verify`, once: **PASS**, native request
  `qbuild-20261024-142943-build-ctest`; build **46/46 edges** (3.2 s), CTest
  **81/81** (98.9 s), total 106.6 s. Both asynchronous operations were collected
  to their actual terminal results. Both native checks used the coordinator queue.

Logs are under this worktree's `build-runs/`: `nation12-all-console.log`,
`nation12-verify-console.log`, `qbuild-test.log`, `qbuild-clippy.log`, and the
latest native `fc4verify-build.log` / `fc4verify-ctest.log`. The q.py Bash wrapper
failed before cargo with CreateFileMapping Win32 error 5; its evidence remains
in `q/20261024-132652-34868.log`. Focused cargo was then run directly and passed.

### Scope, questions and next steps

No new questions filed; NQ1–NQ4 are answered and implemented. No ABI exports,
C API, plugin wiring, ABI version or coverage-map edits. Game runs: none;
shadow exactness and reference-warning measurements are not applicable to this
portable-core change. No request remains outstanding.

The coordinator can land the READY entry and claim the coverage rows after
review. Future native adapter/shadow work uses the existing integration boundary
documented in `engine/core/src/nation/README.md`.
