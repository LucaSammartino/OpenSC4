| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-coverage-functions4: exact function coverage | Codex, `tools-coverage-functions4` | ready; 25 focused tests OK, 232 full-suite tests OK (1 skip), deterministic accounting; zero pilot credit | Coordinator review/land tooling capability |

## Delivery

Implemented in `9ec5bc8c`; generated reports and gate milestone in `04c795dd`. This is the first new
function-stage accounting software capability authorized by the copied Root design and independent Paths
content/capability approvals under `build-runs/evidence-run4/`. The earlier completed owner-audit report remains
unchanged and Directory/HTML remain withheld. No engine, ABI, defaults, names, exclusions, owner rules,
body boundaries, coverage-map rows, regress/shadow inputs, guard or coordinator state was edited.

`tools/coverage_functions.py` validates the tracked `tools/coverage-functions.json` ledger. Only specified and
implemented claims are allowed. It binds exact entry/expected owner/original artifact identity and current
normalized contract, production, assertion, qualification and retained validation hashes; rejects duplicate,
malformed, stale, missing, escaped, foreign, library, unowned, shared, folded and overlapping-body claims;
and fails the measurement closed with diagnostics. Whole-function normal-domain completeness, reviewed
decision/effect correspondence, actual production/assertion symbols and source-matched PASS provenance are
mandatory for implementation. Presence and hashing do not establish semantics or replace independent review.

The shared resolver unions accepted legacy flags with entry flags once per actual entry. All report tallies,
area/subsystem/affected-owner displays, audit TSV and treemap use these flags. Owner-only baseline and explicit
additions stay visible; new entry coverage never promotes an owner to `yes`. Mixed owners split into treemap
boxes by actual entry stage. Verified/replaced/resync policies remain unchanged. Full format and admission
limitations are documented in [COVERAGE.md](../COVERAGE.md).

## Actual validation

No engine/CMake/cargo/clippy/CTest/game gates were run; those are n/a for this tooling lane. All commands ran
in this reserved detached worktree. q.py could not start Bash (`E_ACCESSDENIED`), so commands used PowerShell
directly. Git printed a packed-refs.lock permission diagnostic during normal detached commits but created the
commits successfully; no ref/config/maintenance workaround was attempted.

- Existing focused coverage tests first: `python -m unittest discover -s tools/tests -p "test_coverage_*.py"`
  printed `Ran 11 tests ... OK` before new fixtures.
- An intermediate 24-test run had one fixture failure: the duplicate-record helper overwrote the first
  qualification and got `stale qualification` before the expected duplicate diagnostic. The fixture was corrected
  to duplicate the record without mutating its evidence. This failed attempt was not reported as a pass.
- Final focused `python build-runs/validate-functions4.py focused` ran the same discover command and printed
  `Ran 25 tests in 1.836s`, `OK`, return 0. Fourteen new synthetic tests use literal 13/29/7-byte expectations,
  independent stage flags, overlap union, order independence, no whole-owner promotion, all evidence invalidation,
  CRLF invariance and exact report/treemap/audit parity. Existing resync/noncode tests are included.
- ONE full actual suite on frozen tooling source: `python build-runs/validate-functions4.py full` executed
  `python -m unittest discover -s tools/tests -p test_*.py`. It printed `Ran 232 tests in 38.609s`,
  `OK (skipped=1)`, return 0: 231 passed, zero failures/errors, one Windows platform skip (POSIX filename syntax).
  `build-runs/functions4/full.json` records all source hashes before/after and `stable: true`; no full rerun.
- `python build-runs/validate-functions4.py reports` ran actual `tools/coverage.py --regress <pinned train115>
  --owners <audit>` and `tools/coverage_treemap.py --regress <same input>` twice each, all return 0. Both report
  runs printed `game code 29536 functions, 5584842 bytes; specified 57.8%, implemented 36.1%, verified 19.7%,
  replaced 18.1%`. Both treemaps printed `5584842 bytes; stage shares 42.0%, 21.8%, 11.3%, 5.2%, 6.7%, 12.9%`.
  Report, SVG and audit files were byte-identical across repeats at source commit `9ec5bc8c`.
- `python build-runs/collect-treemap-functions4.py` retained the exact integer collect payload (no third SVG).
  `python build-runs/check-functions4-results.py` independently parsed published-before/generated headlines and
  emitted audit flags, compared the exact treemap payload, fixed denominator/exclusions and empty eligible-entry
  inventory; it printed `verdict: PASS`, `deterministic: true`. Results live in
  `build-runs/functions4/{reports,treemap-payload,independent-check}.json`. `git diff --check` was clean.

## Measured accounting

| Stage | Before = owner baseline = after = audit functions | Before = baseline = after = audit = treemap bytes | Added entries/bytes |
|---|---:|---:|---:|
| specified | 14602 | 3226906 | 0 / 0 |
| implemented | 8982 | 2018463 | 0 / 0 |
| verified | 4325 | 1099943 | 0 / 0 |
| replaced | 3899 | 1012661 | 0 / 0 |

The unchanged denominator is **29536 functions / 5584842 bytes**; unchanged exclusions **6100 / 924106**.
The ledger has no eligible entries, so independently enumerated actual increments are exactly zero.
Mechanical unchanged Attribution measured the three publicly supplied pilot addresses 00473B50/00473B70/00473B90
as 32/25/19 bytes, all `cSC4GameDataRegistry`, rule `vtable` (`build-runs/functions4/metadata.json`). These weights
are not credits. [Registry review](../coverage-functions/registry-review.md) records current public §3.2 rules,
production symbols and literal null/full-width/zero-context assertions. The copied successful engine report
references source-matched qualification/raw logs that were not supplied here as tracked public proof; no
independent exact-entry numeric qualification is published. Every pilot, including specified-only, is withheld.

## Retained normalized SHA-256

| Artifact | Full SHA-256 |
|---|---|
| full tools log | 881f7d7aef73b9f5596c5f4681344736eeb7123d65dff0653f8adf5eebe84fb8 |
| focused tools log | d08044f19737c51f3e9bdce99bc88884cdf686645c800c9da84031883ddfbe49 |
| coverage.py frozen source | 5da83345c5cd07dd7a1c93c6f82ba44d8d121f5ca3002bf7e5ac67697c118a56 |
| coverage_functions.py frozen source | ce51951bc1a6790d76fce4b7242fcf76c92ef2a29c4a3e9bd399c08561bf25fa |
| coverage_treemap.py frozen source | 8f80bbbcb5fab2a41da7db96d964c00d2801502d21e08561798be2b02c47a23e |
| test_coverage_functions.py frozen source | a5b92256aac8a315aef285b2b6a2faa73288f36d0a9f5e48313c5b23671eae89 |
| empty tracked ledger | 0f9a6c888ee4e9aeaadb89d870321f1c7cb0ab438c1c0d73394607a000800a45 |
| pinned regress-train115.out | 1aedd36d983654057265dc3daf57039f8ab7eeb399b8e5860852c511d62094b6 |
| generated coverage-report.md | c02203611fae009ad008646444815764cf4fc7918f0b5d9ace177a2698e877ab |
| generated coverage-map.svg | 30c071bdabbda568e2832c5fb9f788a77bf3bcf2bad730ffec6241cdfcea4d33 |

## Questions and next steps

Questions: none. Game runs: none. ABI exports changed: false. Ready to review and land the tooling capability
with its empty ledger; no numeric-credit or engine change is requested. A future nonempty ledger must publish
and independently review current whole-function correspondence and actual stable source-matched provenance.
No further lane implementation or validation is pending.
