| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-utf8-validation4: standalone UTF8IsStringValid | Codex, `re-utf8-validation4` (reserved detached worktree) | ready: docs, complete normal-domain contract | Coordinator may land the docs; proposed fixtures remain unexecuted. |

## Report (2026-10-23)

Published [utf8-validation.md](../specs/utf8-validation.md), the first dedicated standalone helper
contract, and linked it from [strings-language.md §2](../specs/strings-language.md#2-framework-string-encoding).
Milestones: `d54674ac` publishes the complete rule; `9c94f743` adds the independent byte ledger.
This final report and coverage checklist complete the lane's sole round (original 1 of 3).

### Findings and scope

- Independently qualified native entry `0091016f..0091030e`: 416 contiguous bytes, 145 exported
  instructions, next function at `0091030f`. Two caller-cleaned 32-bit stack arguments; AL Boolean.
  No calls, input/global writes, optional outputs, error index or consumption return channel.
- Explicit count, NUL ordinary, unsigned byte classes, five signed width guards, ascending
  continuation reads, lower-bound checks after continuation reads, first-failure exit and finite
  whole-span success. Supported domain: readable count 0..7fffffff, nonwrapping address addition.
- Accepts 1–6-byte forms, surrogates and above-modern-Unicode forms. Exact restrictions are C2
  minimum lead and E0/A0, F0/90, F8/88, FC/84 first-continuation bounds. No modern-decoder oracle.
- Primary export/byte assembly queried first, all pages read. No live REA needed, no decompiled
  function body used (decomp index/name hits served navigation only). Caller studied solely for
  admission and stack cleanup, not its other algorithms or dependencies.
- File version 1.1.641.0; PE32/x86 machine 014c, magic 010b, base 00400000; working copy/export
  share SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
- Current-doc and all-ref content-history searches found no earlier dedicated helper publication.
  `re-strings4` explicitly leaves encoding helpers unstarted. The named inherited
  `re-string-storage-final4` report is absent from this snapshot; the task explicitly states its
  cRZString-only exclusion. No String/LanguageManager/Utility/converter credit reopened.

### Validation

- **PASS:** `python -X utf8 tools/doccheck.py check` and `links`;
  `python -X utf8 tools/questions.py check`; `git diff --check`.
- **PASS, static analytic only:** ignored `build-runs/check-utf8-evidence.py` verifies PE/export
  identity, the exhaustive 256-byte lead partition and continuation predicate, all 256 possible
  first-continuation values under each extra bound, 107 literal cases across 92 rows, all 15
  truncation width/count pairs, all 15 continuation positions with both rejected sides, bounded
  S-case counts and changed-spec local link targets. No whole-input host validator is an oracle.
- Literal results and read-order traces are **PROPOSED / UNEXECUTED**. Native executions **0**;
  portable-validator executions **0**. No game requests/runs, no shadow claim.
- Cargo/clippy/ctest: **n/a**, docs-only RE lane; no engine or implementation/test sources read or changed.
- Questions filed/answered: **none**; no index regeneration needed. Null-pointer, negative/large
  count, wraparound and inaccessible-buffer domains remain expressly unknown/unsupported, rather
  than fabricated false/success guarantees. Normal readable-buffer branches are fully specified.

### Tool errors and limits

The q.py wrapper failed with Bash E_ACCESSDENIED; bounded PowerShell replaced it. A qgrep call
failed on cp1252 output encoding; Python UTF-8 mode resolved it. A literal PowerShell glob path
was rejected by rg and replaced with `--glob`. Reading the inherited final-storage report failed
because it is absent. Git printed `packed-refs.lock: Permission denied` during successful detached
commits; HEAD advanced normally and no shared ref/config was changed. These errors do not alter
the native evidence. No RE server or native-program launch, native probes or new tooling installation occurred.

No ABI proposals/exports/version changes. Only the two specs and this report are committed.
Ignored evidence and the own-lane queue live under this worktree's `build-runs/`. The manifest is
`C:/Users/tlori/FreeCity4/build-runs/run4-re-utf8-validation4-r1-manifest.json`; guard statuses
remain within this lane's budget/call cap. No follow-on round or broader encoding work is requested.

Next: coordinator lands the docs; implementers can derive portable verification later from the
published rules and proposed literal ledger. Native execution remains unclaimed.
