| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzframework-gzf5: GZF10/GZF11 contracts | Codex / re-gzframework-gzf5 | ready: both answered; 36 added vectors; doccheck and questions check pass; ABI unchanged | core-gzframework5 can apply corrected contracts; broader GZF1–GZF9 remain open. |

## Report

Answered GZF10 and GZF11 in `docs/specs/gz-framework-core.md`, moved both original
entries with dated answers to `docs/specs/questions/answered/gz-framework-core.md`,
and regenerated `docs/specs/QUESTIONS.md`. Corrected the ambiguous buffer Unlock
wording in `docs/specs/span-render-8.md`. Added prose vectors GZC39–GZC74: 36 new,
74 unique vectors total. No engine, plugin, C API, ABI tables or coverage map edits.

### Contracts implementers need

- Ordered participants use descending signed 32-bit priority; distinct ties keep insertion order.
- Buffer Unlock removes only supplied mask bits and wraps 16-bit nesting. Fresh color key is disabled;
  its getter does not write output. Buffer QI/lifetime, including count-zero underflow, are specified.
- Support QI accepts only `0x452294AA`; final Release deletes without guarded Shutdown.
- Framework getters borrow embedded/helper/application pointers; slot 22 returns a byte flag.
  Stream getters lazily retain null streams; SetStream clears before replacement QI.
- Basic and propagated debug values are separate; propagation requests a signed decimal environment value.
- Logging owner thread is captured by Init, not construction. Public output is printf varargs;
  Output uses level zero and OutputLevel uses its argument. Name-set accessors are borrowed.
- Failed directory setters preserve prior cache validity while replacing the cached path.
- COM enumeration distinguishes borrowed cancellable callbacks from counted AddRef'd arrays.
  SetServiceRunning only stores a byte.
- Command identity/lifetime, formatted-entry first-vararg forwarding, registration replay and
  idempotent removal are specified, including null-generator recursion as an unchecked quirk.
- AddOutputStream can return true when duplicate-name backend registration fails. DumpOutputLevels
  and output initialization/null guards now have explicit contracts.

### Evidence and validation

Used main-checkout read-only `tools/re_query.py` first, then relevant class decompilation and
export `asm --bytes` pages. The spec cites entry/range addresses for the recovered rules.
No REA calls were made: this round was explicitly export/decomp only. No new comparison of
export instructions against executable bytes is claimed. Initial missing/folded-name queries
were resolved using exact addresses and nested class paths, not treated as recovered evidence.

The engine Owner closure table was not read because this lane is expressly forbidden to read
`engine/`; its implementation-facing gap summary in `docs/lanes/core-gzframework5.md` supplied
the closure scope. The report above identifies the fixture assumptions that need correction.

Final checks: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS;
`git diff --check` PASS after removing an extra EOF blank line. Vector-ID audit: 74 unique,
GZC39–74 all present. Cargo/clippy/ctest: n/a (spec-only). Game runs/shadow: n/a; no replacement
safety or native execution verdict claimed. No proposed ABI rows or C ABI export changes.

Milestones: `d031066a` GZF10; `4d047b70` GZF11. Git commits succeeded on detached HEAD despite
a shared `packed-refs.lock` permission warning; no shared refs/config were changed.
Questions answered: GZF10/GZF11; questions newly filed: none. Broader GZF1–GZF9 remain as scoped.

## Handoff

- Goal complete: land spec answers and pass them to core-gzframework5.
- Milestones: d031066a (GZF10), 4d047b70 (GZF11); final report/check cleanup follows those.
- Next: coordinator publishes actual detached HEAD to reserved branch and consumes the lane queue entry.
- Apply GZC39–74 when updating core fixtures; no game requests are outstanding.
- Remaining owner work is GZF1–GZF9, outside this round; no engine owner closure is asserted.
- Queue: build-runs/merge-queue/re-gzframework-gzf5.json; kind re, ready, ABI false, no game runs.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-gzframework-gzf5-r1-manifest.json.
