| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-draw-context-contracts4 (DC3/DC8 native contracts, round 2) | `re-draw-context-contracts4` / (RE spec lane, detached worktree) | blocked: parked by coordinator STOP RE; proof 68cd0285, 97a08d63, f2150aaf; DC8 answered DIFFERS; questions check and diff check passed; cargo/clippy/ctest and game n/a; no ABI change | DC3 only: attribute native stack mutation or prove no population; no further RE work without coordinator release. |

## Handoff

- PARKED: guard directed STOP RE, prioritizing verified/replaced work; no further RE round is authorized.
- Goal remaining: DC3 stack mutation attribution or proof of no population. Do not infer Push/Pop signatures.
- Prior proof: a0a0d989 lifecycle/combiner bytes; a81f458d inventory/cache order; 14f8afbe DC4–DC7/numerics; 44e79acd report.
- Done: §8.4 attributes raw-matrix receiver/signature and four callers; exact grouped four-term arithmetic, explicit zero products, binary32 stores, transpose/load and failure/retry order.
- Done: §8.3 proves all nine compact rotation groupings, asymmetric translation spills, scale/flag branches, counter copies, matrix expansion and failure/retry mutation order.
- Done: DC8 moved to answered with full rule; §13.2 adds DC-V15–V26 cancellation/spill/scale/flag/state/failure vectors (26 total independent vectors).
- Commits: 68cd0285 raw-matrix contract; 97a08d63 compact grouping/spills; f2150aaf DC8 answer and vectors.
- Gates at f2150aaf: questions index regenerated, questions check ok, git diff --check passed; cargo/clippy/ctest/game n/a.
- Native offset scan: 963 references to +0x1EC/+0x1F0/+0x1F4 in 311 export bodies; not all refer to this owner.
- Within the owner region, only constructor writes (007D6271/6277/627D), default replay reads (007D5200/5206/521B) and destructor read (007D5BB3) were found.
- This is bounded negative evidence, not proof against all foreign/aliased writes; DC3 stays open.
- Ignored evidence: build-runs/dc2-offset-owners.txt labels 248 non-stack/non-call candidate bodies; dc2-audit.py, dc2-symbolic.py, dc2-vectors.py reproduce audits/calculations.
- SetDefaultRenderState has two direct callers: 005CC7DD construction draw and 005EEDB2 burnt-building draw; zero exported data references.
- Container helper direct-call chains: 5AB0 from 5D60; 5D60 from 5EB0; 5EB0 only Init at 638A (receiver owner+E0, stages); 5C20 only constructor at 62DC (16-byte colour records). No stack mutation follows from these helpers.
- Next if released: classify the remaining whole-export offset candidates by actual receiver, including aliases; then decide whether DC3 can be answered. Minor wording follow-up: new vectors say "bit 2" where mask 0x02 is intended.
- Raw-matrix callers: 005DCE50 border masks; 006572B0/006573B2 underground connection; 0065B5DA underground occupant.
- Stack replay is non-popping; no Push/Pop signatures established. Whole-byte compact-transform flag tests already proved.
- Unsupported context Begin/End request already withdrawn (§1.3); callbacks own renderer brackets. DC4–DC7 stay untouched.
- No engine reads, runtime/coverage/ABI claims or game requests. Worktree stays detached; no ref updates.
- q.py Bash wrapper fails E_ACCESSDENIED; use bounded PowerShell commands.
- Active manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-draw-context-contracts4-r2-manifest.json.

## Round 2 report

Published exact native raw-matrix attribution, pointer argument/callers, grouped arithmetic, explicit zero
products, binary32 stores, transpose and driver/mutation/failure/retry order in §8.4 (68cd0285).
Published every compact rotation grouping and the asymmetric translation spills: x/y spill after scale,
z before scale. Whole-byte flag branches, model/composed counters, scale store, matrix expansion and
driver/failure/retry order are in §8.3 (97a08d63). Added DC-V15–V26, bringing source-derived independent
vectors to 26, and answered DC8 DIFFERS (f2150aaf). DC4–DC7 were not reopened. Unsupported owner
Begin/End remains withdrawn; no Push/Pop API was invented.

DC3's raw-matrix residual is closed; its stack-population residual remains open in
docs/specs/questions/draw-context.md. The whole-export offset scan found only constructor/replay/destructor
uses within the attributed owner region, but unrelated receivers and possible aliases prevent an honest
whole-program non-population claim from this scan alone. The lane guard then parked the lane with an
explicit STOP RE direction. No further investigation or merge-queue write was attempted after that stop.

Validation at f2150aaf: question index regenerated, questions check ok, git diff --check passed.
No engine files read/changed, ABI proposals, runtime/coverage/replacement claims or game runs.
REA was not needed. The local export, bounded decompilation and executable data supplied all evidence.
Git commits succeed despite a packed-refs.lock permission warning; HEAD remains detached as required.

## Prior round report (historical; its DC8 residual is superseded above)

Substantive native evidence is committed in `a0a0d989` (lifecycle and combiner bytes), `a81f458d`
(inventory, stage defaults, ordered cache/driver effects), and `14f8afbe` (numeric/shade contracts,
14 independent source-derived vectors and question disposition). The round is **wip**, not complete.

The Begin/End request is withdrawn: renderer pass brackets call list callback receivers and pass the
non-virtual draw context as their argument. Constructor, Init, shutdown and storage-destructor bodies
are attributed. Driver endpoints are separately mapped against DX7 table `0x00AC4660`. Export labels
for `0x007D30D0`/`0x007D32D0` incorrectly say renderer; callers prove context receivers. This report
and the spec correct the attribution without editing maps or ABI tables. No ABI proposal file was needed.

Complete 12-byte owner combiner records are published for construction and Init, plus the distinct
stage-default record. Explicit word stores prove their zero tails. Stage texture starts `0xFFFFFFFF`,
coordinates 0, wrap/filter caches −1, flag false and saved matrix identity; three padding bytes remain
explicitly unspecified. Owner default shade is white from executable data, not constructor-zero.

The spec distinguishes binding without selection/equality tests; equality-gated coordinate/wrap/filter
selection; duplicate transform selection; suppressed-cache updates; ungated clear/reset/combiner paths;
and stage-above-1 environment-mode behavior. Driver-return failures normally advance caches, so equal
retry can be silent. There is no native first-failure accumulator or guaranteed exception cleanup.

Lighting slot 10 populates default shade. Model-light slot 11 has a checked AL result and default fallback;
SetModelShade sets override without overwriting default. Grey preserves a prior mesh alpha multiplier.
All material texture-matrix entries are specified, including zero z/homogeneous entries; the zero-fill
provenance is identified rather than inferred from omitted decompilation. Alpha packing uses an x87
bias and binary32 store, with nearest/even conditional on the ambient control word; 0.7f packs to 10.
Material threshold conversion uses binary64 `0x3F70101010101010`, followed by a binary32 store.

DC4–DC7 moved, with original entries and dated DIFFERS answers, to
`docs/specs/questions/answered/draw-context.md`. DC1/DC2 are untouched. DC3/DC8 remain in the open
question file with precise partial answers and residuals. No implementation, replacement, runtime,
coverage or shadow-equivalence claims are made; engine files were neither read nor changed.

Validation: `python tools/questions.py index` regenerated the index; `python tools/questions.py check`
reported `questions: ok`; `git diff --check` passed. Cargo/clippy/ctest and game runs are n/a for this
spec-only round. REA was not needed; evidence came from local export queries, bounded decompilation
reads, executable bytes and PE section metadata. The Bash q.py wrapper failed E_ACCESSDENIED; native
PowerShell worked. Git reported a read-only packed-refs lock warning but created each detached commit.

## Remaining work

1. DC3: inspect actual memory accesses to stack begin/end/capacity `+0x1EC/+0x1F0/+0x1F4` to attribute
   mutation sites/signatures, or establish non-population. Current proof only establishes non-popping
   top replay at end−8, empty fallback and unilateral reset ignoring the stack. Do not invent Push/Pop.
2. DC3/DC8: finish body `0x007D4B80` (called by border-mask and underground draws); it is a raw-matrix
   transform/load path, not a stack method. All body addresses/callers are available through re_query.
3. DC8: specify exact x87 grouping/spill points for mixed compact transforms at `0x007D3400`, with
   cancellation/scale/whole-byte-flag vectors. Whole-byte identity branches and final scale-bit test
   are proved in §8.3, but the current algebraic summary must not be treated as bitwise arithmetic.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-draw-context-contracts4-r1-manifest.json`.
