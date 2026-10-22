| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lighting-contracts4: LT5–LT11 native lighting contracts | Codex / `re-lighting-contracts4` (RE docs, detached supervisor checkout) | wip; LT5–LT9/LT11 answered, LT10 partial; 43+2 owner bodies inventoried; docs checks PASS; ABI unchanged; cargo/clippy/ctest and game n/a | 1. Prove LT10 direction/flag composition and rounding at 007D8C80. 2. Implementers reconcile answered contracts before complete-owner credit. |

## Report (2026-10-22)

Changed only the lighting spec, its active/answered question files, generated question index and this lane
report. No engine files were read or edited; no tools/maps/ABI tables changed. No ABI proposal is needed.

The individual inventory is 43 manager bodies / 21,714 native bytes and 2 image bodies / 517 bytes.
Six attributed helpers are identified separately; shared AddRef/CRT and foreign terrain/image/property/message
services are dependency boundaries. Inventory completeness does not claim complete-owner behaviour credit.

Answered LT5–LT9 and LT11 in `docs/specs/questions/answered/lighting.md`, with native rules in
`docs/specs/lighting.md` and regenerated `docs/specs/QUESTIONS.md`:

- LT5 DIFFERS: zero initial reference count; black announcement/cache values, white current light,
  fractions 1/0, initial sun (1,-1,-1), partially unwritten instance state, initial shared property table.
  No universal pre-city fade result; successful QueryInterface acquires one reference.
- LT6 MATCHES: Euclidean RGB distance, blue+green then red squares, x87 intermediates, strictly above
  binary32 threshold; equality and unordered tests fail.
- LT7 DIFFERS: minimum scans vector storage including padding, not unused capacity. Image/model accesses
  are unchecked. Missing map Load does not fail Init; no transactional copy recovery. Explicit Shutdown
  frees maps/unsubscribes; destruction alone does neither.
- LT8 DIFFERS: epsilon is binary64 0.001, upper replacement rounds binary32; fixed X rows of Z floats at
  every rotation, sx=0 nonfinite drift skips with retained rectangle. No-change rectangle is (-1,-1,0,0).
- LT9 MATCHES: inclusive changed vertex endpoints are rotated verbatim without expansion/clipping;
  deferred no-change sends neither terrain notification. Recipient-side expansion remains its own contract.
- LT11 DIFFERS: complete six-float box/four-integer rectangle procedure, quadrant-specific ordered corner
  probes, continuous first/second-probe edge and height, independent final-axis probe, strict stopping,
  inclusive retained stop vertex, no fractional cross-axis drift or final clipping, true return.

LT10 stays open. Section 5.8 newly proves **count-first** argument order, separate transform/output
storage, basis helper including the vertical fallback, signed **binary64** 0.01 determinant comparison,
NaN proceeding past that test, and untouched output on count-zero failure. Full model-frame direction,
flag-dependent coefficient composition and accumulation/cofactor/composition rounding remain unproved.
Do not implement that gap from the older inferred overview or promote complete-owner coverage.

## Evidence and checks

Primary evidence is the local original Windows export's instruction bytes and executable data, queried
through `re_query.py`; decompilation guided navigation. Numeric/lifecycle/rectangle claims cite addresses in
the spec. No REA request, game run, network request or original-method provider was used.

Nine independent conformance examples were added as spec text (vectors 10–18). Local scratch check
`build-runs/check-lighting-docs.py` passed inventory uniqueness/body sizes, numeric discriminators and
six-answered/one-remaining accounting. Question-index regeneration is idempotent. `git diff --check`
passed across all three substantive commits. Cargo/clippy/ctest and runtime/shadow/replacement checks are
n/a for this docs-only round; no measured in-game exactness is claimed.

Substantive commits: `69fd2874` inventory and numeric/storage boundaries; `82a5c8c2` lifecycle, statics,
terrain extension and message rules; `40622e51` question reconciliation and partial projection evidence.
The supervisor-reserved detached checkout was retained. Git printed a packed-refs lock permission warning
during commits, but each commit succeeded and the resulting HEAD/clean tracked state were verified.

## Next steps

Prove LT10 using `007D8C80–007D9813`: inverse setup `007D8D13–007D8DEA`, direction/basis setup
`007D8E36–007D8EC5`, accumulation/determinant `007D8F50–007D91E4`, then composition and float stores
through `007D9801`. Recover all transform flag combinations and exact spill boundaries before closing it.
Implementers can reconcile the six answered questions now; portable safety policies for invalid reads and
unset pre-city fields must remain explicitly distinguished from native defaults. This round ends WIP;
queue state is wip and no further round or self-restart was initiated.
