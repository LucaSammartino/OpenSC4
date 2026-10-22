| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-draw-context-contracts4 (DC3–DC8 native contracts) | `re-draw-context-contracts4` / (RE spec lane, detached worktree) | wip: native proof a0a0d989, a81f458d, 14f8afbe; DC4–DC7 answered DIFFERS; questions check and diff check passed; cargo/clippy/ctest and game n/a; no ABI change | (1) Attribute stack mutation or establish no population; (2) finish exact mixed-transform and raw-matrix arithmetic for DC3/DC8. |

## Report

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
