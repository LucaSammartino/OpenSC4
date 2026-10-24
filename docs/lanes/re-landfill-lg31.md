| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landfill-lg31: public zone verification boundary | Codex / `re-landfill-lg31` (RE, detached worktree) | ready: LG31 answered; four boundary vectors; doccheck PASS; no native zone comparisons or coverage credit; ABI unchanged | Coordinator may land docs; retain partial landfill verification |

## Report

- `0a85bd650`: landfill §7.4 answers LG31 in one contract paragraph. No supported published public-interface route exists for independently invoking native zone queries, mutations or record Read/Write on controlled inputs. Borrowed acquisition and balanced lifetime retention do not provide those operations.
- Landfill §11.1 adds four prose conformance vectors (LG31-B1 through B4): acquisition, authored zone records, indirect developer/occupant evidence, and question-closure accounting. These are admission/accounting expectations, not measured native results.
- Moved LG31 to `docs/specs/questions/answered/landfill.md`, marked MATCHES, and regenerated `docs/specs/QUESTIONS.md`. No new questions. The brief's duplicated `docs/specs/` paths resolve to the existing landfill files; no duplicate paths were created.
- Read glue-landfill-shadow12's questions/next-steps first: the supplied report contains no Owner closure table or `open:` rows. Its explicit LG31 boundary is the relevant blocker. Closing the contract question does not unblock independent native zone tests or make landfill fully verified; the coordinator can retain the measured developer/occupant results with this limitation.
- Evidence is the already-published landfill §§1, 4.4.9, 6.2, 7.1, 7.4 and 9 and absence of zone rows in the main checkout's ABI tables. Export queried first; no new binary investigation or REA was necessary for this literal contract question. No engine, plugin, capi, ABI tables or coverage map read/edited beyond the permitted ABI-table review; no proposed ABI rows.
- Validation: `python tools/questions.py index` succeeded; `git diff --check` clean; direct foreground `python tools/doccheck.py check` **PASS** (`build-runs/doccheck-re-landfill-lg31.log`). Cargo/clippy/CTest n/a for spec-only changes. No game requests or runs; no native comparisons or coverage credit claimed.
- Tooling detail: the first doccheck launch through q.py failed before executing Python because Git Bash could not create its Windows file mapping (`build-runs/q/20261024-155108-24208.log`). The subsequent direct PowerShell invocation passed. The milestone commit was created successfully; Git subsequently reported a denied shared `packed-refs.lock`. No shared-ref changes or Git maintenance were requested.

## Handoff

- Goal: resolve LG31 as a published-contract decision; spec text only.
- Done: `0a85bd650` contains spec, vectors, archived LG31 and regenerated index; doccheck passed.
- Next: supervisor publishes detached commits to reserved branch; coordinator lands docs while preserving unsupported independent zone verification.
- No outstanding game requests, new questions, ABI proposals or implementation work in this lane.
