| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nation-nq4: tutorial directory case mapping | `Codex` / `re-nation-nq4` (RE, detached HEAD) | ready: NQ4 DIFFERS; ten spec vectors; doccheck and questions check PASS; ABI unchanged; game n/a | 1. Land spec closure. 2. core-nation10 replaces provisional ASCII-only folding with §6.4 behavior. |

## Report

Spec milestone: `06773c23e` (initial lane handoff `0e581e39d`). Nation §6.4 now defines UTF-8
directory names and active CRT LC_CTYPE lowercase mapping. §5.2 supplies ten conformance vectors;
§8 clarifies the comparison boundary. NQ4, obtained with `git show core-nation10:docs/specs/questions/nation.md`,
is archived in `docs/specs/questions/answered/nation.md` as **DIFFERS**. The regenerated index has
**0 open, 4 answered, next NQ5** for nation.

The provisional core choice of preserving non-ASCII bytes differs from the original. C locale retains
accented capitals; en-US lowercases them. The conversion is UTF-8 to 16-bit characters and back, with
observable malformed-input prefix truncation and supplementary-character truncation. Final comparisons
use equal byte lengths and equal bytes, with no normalization. Init and reset fold catalog names and
compare registry results verbatim; the ordinary registry folded its defaults and keys when loading them.
Preferences still compare display-name bytes case sensitively. The spec covers stale registry keys after
a locale change, without inventing automatic remapping.

Source evidence came only from the main checkout's read-only export and bounded decomp reads, queried
first with `tools/re_query.py`. Numeric widths, thresholds and conversion order were verified with
`asm --bytes`: MakeLower `0x0090EAC5`, fixed conversions `0x009113EF` / `0x00911358`, encoding-8
decoder `0x00910E18`, CRT lowercase `0x009F4C35` / `0x009F4BD4`, language locale change
`0x00928021`, registry folding `0x004B7998` / `0x004B7BB2`, and reset comparison
`0x00498FE6–0x00499015`. No REA call was made; the configured session has no binary open.

Validation collected to terminal success on 2026-10-24:

- `python tools/questions.py index`: regenerated the index; only the nation row and total changed.
- `python tools/questions.py check`: **questions: ok**.
- `python tools/doccheck.py check`: **doccheck check: ok**.
- `git diff --check`: **PASS**.
- Cargo, clippy and CTest: **n/a**, spec-only lane. Ten vectors are written expectations, not executed tests.
- Game runs: **none**; shadow equality is unmeasured. No ABI export changes or proposed ABI rows.

Scope stayed in nation spec/question documents, the generated question index and this lane report.
No engine/, plugin, capi, ABI tables or coverage-map edits. The engine Owner closure table is under
engine/ and was not read because the task explicitly forbids reading that tree; the published
core-nation10 lane report and its exact NQ4 entry supplied the open contract. Local commits succeeded
despite Git printing a denied shared `packed-refs.lock` attempt; no refs/config maintenance was attempted.

## Handoff

- Goal: close core-nation10 NQ4 with behavior and conformance vectors, no code edits.
- Done: spec/archive/index milestone 06773c23e; all requested document checks pass.
- Next: supervisor publishes detached commits to reserved branch; coordinator lands queued full tip.
- Implementer follow-up: replace ASCII-only directory fold using nation §6.4 and §5.2; registry results remain verbatim.
- No outstanding RE questions, game requests, ABI rows or running checks.
