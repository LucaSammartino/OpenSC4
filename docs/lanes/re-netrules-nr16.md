| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netrules-nr16 (network rule observation contracts) | Codex / `re-netrules-nr16` (RE lane, detached HEAD) | ready: NR16 answered; 9 prose vectors; doccheck and questions check PASS; no ABI export change; no game run | (1) Publish ABI proposals; (2) reconcile pending shadow-lane NR16 filing; (3) resume glue-netrules-shadow5. |

## Report

Answered NR16 in [network-rules.md](../specs/network-rules.md) §2.5 and added nine prose conformance vectors,
§15 tests 36–44. Corrected §2.4’s output argument and the rule size (0x20 bytes including its eligibility tail).
The answer covers the requested native call signature and return channel, borrowed output ownership, tree/header
and element layouts, unsigned and equal-key traversal, neighbourhood layout/accessors/cache, global intersection
map layout, executable checks, observation spans, loading epochs and synchronous capture lifetime.

The original returns a boolean in AL and a pointer to the matched output-vector descriptor through its second
argument. Matched-empty succeeds with a non-null descriptor. Equal-key order survives tree balancing and must be
preserved. Two observations matter for a record-only shadow: the process flag is set before ReloadRules starts,
and GetCellInfo can populate world-cache entries. Therefore readiness follows observed load completion, and
capture follows the original’s actual queries without native prefetch or a second live evaluation. Replay uses
detached semantic copies and reports missing captured inputs separately from exact results and mismatches.

Evidence was obtained from main-checkout re_query.py instruction pages, the named class decompilations and direct
PE data reads for the neighbourhood vtable. Numeric widths, offsets, return channels, node allocations and entry
spans were checked against instructions; citations are in the spec. Examined working executable SHA-256:
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
REA was not used; no session failure or fallback occurred. No original code or assembly listings were copied into
the spec. No engine, plugin, capi or coverage files were read or edited; shared ABI files were read only.

NR16 was present only in the reserved glue-netrules-shadow5 worktree (its question commit `4965eb6a`), not in this
base or main. That lane’s report had no Owner closure table; the explicit NR16 request supplied the closure scope.
The complete question is now in [the answered archive](../specs/questions/answered/network-rules.md), marked
**ANSWERED 2026-10-23 (re-netrules-nr16): DIFFERS**. The generated question index now reserves NR17 as next free;
NR6 and NR7 remain open and outside this task. If the coordinator lands the blocked lane’s question-only commit,
remove its now-obsolete open NR16 entry and regenerate the index rather than reopening this answer.

ABI publication proposals (gitignored artifacts in this worktree):

- `build-runs/abi-rows-re-netrules-nr16.tsv`: two neighbourhood vtable rows in docs/abi/vtables.tsv format.
- `build-runs/abi-contracts-re-netrules-nr16.md`: direct-call signatures, private field tables, ownership and
  version/observation-site facts for docs/abi/direct-calls.md and layouts.md.

The coordinator must promote these interface facts before engine glue depends on their publication in docs/abi.
They add no C ABI export and do not bump FC4_ABI_VERSION. The engine lane still owns detour implementation,
installation checks, live capture completeness, gate and real-city verification through 2032. This RE result
makes no shadow-exactness or patch-safety-in-game claim.

Validation: `python tools/questions.py index`, `python tools/questions.py check`, `python tools/doccheck.py check`
and `git diff --check` passed. Cargo, clippy and CTest are n/a for this documentation-only change. The nine vectors
are specified expectations, not executed engine tests. Game runs: none. No native verify or game request is pending.

Milestones: `887c9616` native observation specification; `9031e4c0` question closure, vectors and passing checks.
Both detached commits succeeded; Git also printed a denied shared packed-refs lock attempt. No shared ref/config
repair or Git maintenance was attempted.

## Handoff

- Goal complete: NR16 native observation contract, archive and nine prose vectors.
- Done: specification 887c9616; closure/validation 9031e4c0; final report committed at queued tip.
- Next: coordinator publishes ABI proposals, reconciles unlanded NR16 filing and resumes the engine shadow lane.
- Facts: no game requests or ABI export changes; no runtime exactness claim. Proposal paths and all measured evidence are above.
