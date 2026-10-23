| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scres-scr5: SC resource open contracts | Codex / `re-scres-scr5` (RE lane) | ready; doccheck PASS | Coordinator integration; core-sc-resources5 can implement all closed SCR domains. |

## Report

Closed **SCR1–SCR5 and SCR7**, all open questions in this spec. The behavior is
published in [sc-resources](../specs/sc-resources.md) sections 2–7; original
questions and dated MATCHES/DIFFERS dispositions are in the
[answered file](../specs/questions/answered/sc-resources.md). Regenerated
`docs/specs/QUESTIONS.md`; no new questions were filed.

Added 38 independent conformance vectors **SCR-V31–68**, bringing this spec to
68 vectors. These are requirements for implementers, not claimed executed
engine tests. They cover grouping/expression quirks, namespace/substitution,
builtin failures, relative includes/recovery, all twelve token wire types,
binary/string boundaries, raw text fragments, overflow/nonfinite formatting,
resource type identity, record BMP behavior and exact 4444 diffusion.

Instruction checks resolved several nonconventional contracts: decimal
exponents use a wrapping integer power of ten; setc emits rotated components;
slot 9 removes an exact variable key; text uint64 input inherits binary reading;
record BMP locks with 0x8040 while TGA uses 0x8080. The spec describes these
literally instead of leaving the core lane's portable assumptions unresolved.
Unsafe malformed RLE and integer-divide-fault contexts have an explicit
portable rejection boundary, without invented native success results.

Evidence: main checkout's read-only export/decompilation; numeric widths,
operation order and constants checked with `re_query.py asm --bytes`.
No REA query was needed. No engine, plugin, capi, tools/coverage-map.tsv,
docs/abi or C export changes; no proposed ABI rows.

Validation: `python tools/questions.py index` completed;
`python tools/doccheck.py check` **PASS**. Earlier-round doccheck usage-error
and guard-blocked attempts were incomplete, not failed checks. Cargo, clippy,
ctest and game/shadow runs: n/a for this RE documentation-only change.

Work commits: 4d687c7b (SCR1 draft), 9128baba (wire/image draft),
3215b0b2 (all answers, vectors and index). The worktree is intentionally
detached; the supervisor publishes the final commit to the reserved lane
branch. A packed-refs lock permission diagnostic accompanied the detached
commit, which nevertheless completed; no ref/config workaround was attempted.

## Handoff

- Complete: all six SCR questions closed, vectors published, index regenerated, doccheck PASS.
- Next: coordinator publishes detached tip and integrates the ready RE queue entry; core lane consumes the spec only.
- No game requests outstanding, new questions or ABI proposals.
- Ignored evidence helpers: build-runs/sections.py and peek.py; original files remain outside this worktree.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-scres-scr5-r2-manifest.json.
