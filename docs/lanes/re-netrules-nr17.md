| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netrules-nr17 (observation boundary bytes) | Codex / `re-netrules-nr17` (RE) | ready: NR17 answered; ABI reference published; five prose vectors; doccheck/questions/diff checks pass; no C ABI exports or game runs | 1. Land NR16 + NR17 and reconcile shadow5's obsolete open filings. 2. Resume glue-netrules-shadow5 implementation and runtime verification. |

## Report

Merged the requested NR16 branch by fast-forward to `6bf9cc16`. Published NR17 and the supporting NR16 native
contracts in [docs/abi/network-rules-observation.md](../abi/network-rules-observation.md), as explicitly requested
by this task. This new dedicated interface document avoids rewriting existing shared ABI tables. It supersedes
the NR16 report's pending-publication prerequisite. It contains the six direct-call signatures, neighbourhood
virtual entries, private layouts, ownership/lifetime constraints, exact observation-site bytes and PE restrictions.
There is no C ABI export change or FC4_ABI_VERSION change.

The export bytes were queried first with main-checkout `tools/re_query.py asm --bytes`, then independently
compared with direct read-only PE file reads. All three spans matched. Whole-instruction spans and continuations:

- Evaluate: 5 bytes, continuation `0x006220D5`.
- ReloadRules: 6 bytes, continuation `0x006253D6`.
- Static cleanup: 5 bytes, continuation `0x006232F5`.

The file fingerprint matches NR16's examined executable. PE machine `0x014C`, timestamp `0x4C12BFB7`,
SizeOfImage `0x007C8000`, ImageBase `0x00400000`, Characteristics `0x010F`, DLL Characteristics zero;
base-relocation directory RVA/size zero. The supported observer requires the original loaded base.
Cleanup's absolute flag operand is retained unchanged in a trampoline; no byte mask or invented rebase is allowed.
The complete callback confirms that cleanup always returns AL=1, writes zero only for a nonzero incoming flag,
and leaves tables allocated. Observer readiness is invalidated on entry even for an incoming zero flag.

[network-rules.md](../specs/network-rules.md) §2.5.6 states these behavioural consequences and fail-closed installation
requirements. §15 adds five prose conformance vectors (45–49): accepted reference/whole spans, independently
changed loaded sites, absolute operand handling, unsupported module base, and cleanup with retained tables.
They are specified expectations, not executed engine tests or an installed-hook safety claim. NR16 capture
semantics, original-thread lifetime rules and incomplete-input reporting remain applicable.

The requested Owner closure table does not exist in shadow5's current lane report. Its report and NR17 question
were read first as the closure checklist: byte-validation reference, cleanup observation span, and ABI publication
are now supplied. NR17 was only in shadow5's unlanded worktree, not our open question file. The complete entry is
archived in [answered/network-rules.md](../specs/questions/answered/network-rules.md), marked
**ANSWERED 2026-10-23 (re-netrules-nr17): DIFFERS**; the regenerated index reserves NR18 next. NR6 and NR7 remain
open and outside this task. On landing shadow5's question commits, remove its obsolete open NR16/NR17 entries
and regenerate the index rather than reopening either answer.

Optional existing-table publication rows are in this worktree's gitignored
`build-runs/abi-rows-re-netrules-nr17.tsv`, using `docs/abi/vtables.tsv` columns. They repeat the two neighbourhood
entries already published in the new ABI document; no shared TSV regeneration is needed to consume this contract.
Read-only verification helper: `build-runs/inspect-nr17-pe.py`. No REA session was used; no REA failure or fallback.
No engine, plugin, capi or coverage-map files were read or edited. No original code or assembly listing is in specs.

Validation: `python tools/questions.py index`, `python tools/questions.py check`, `python tools/doccheck.py check`
and `git diff --check` passed. Cargo/clippy/CTest are n/a. No game run, pending request, 2032 verdict or shadow
exactness claim. The engine lane owns hook implementation, native tests, gate and real-city verification.

Milestone: `99773654` publishes the ABI/spec, archive and five vectors. Git's attempted shared packed-refs lock
was denied during merge/commit, but actual detached HEAD advanced normally. No shared-ref repair, branch switch,
Git maintenance or push was attempted. The supervisor publishes the detached tip to the reserved branch.

## Handoff

- Goal complete: NR17 byte reference, cleanup contract and native ABI publication.
- Done: merged NR16 at 6bf9cc16; publication/spec/archive/vectors and passing checks at 99773654.
- Next: supervisor publishes queued detached tip; coordinator reconciles unlanded question filings and resumes shadow5.
- Facts: fixed base 0x00400000; spans 5/6/5; no relocations; cleanup retains tables but invalidates readiness.
- No game requests, native hook installation, runtime exactness claim or C ABI export change.
- Optional TSV rows and read-only evidence helper are named above; ABI facts are already in the committed document.
