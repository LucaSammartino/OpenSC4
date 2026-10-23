| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-automata-controller-lifecycle4: inherited base controller lifecycle | Codex / `re-automata-controller-lifecycle4` (RE lane; detached HEAD, supervisor publishes reserved branch) | blocked: coordinator STOP before final publication; two proof milestones committed; ABI unchanged; no game runs | Preserve commits and uncommitted final fixture refinements; no queue written. |

## Handoff

- STOP enforced 2026-10-23 09:16:47: user stopped goal; halt new work and report status.
- Real proof commits: `d1f28698` Read/facing/radius and `7e373dd3` optional distance/intersection/state/behavior, 17 new proposed/unexecuted fixtures.
- Existing R1/R2 commits and ignored evidence remain preserved; AT22 stays partial.
- After second milestone, R8/R9 coordinates refined to exact 3–4–5 / 5–12–13 distances in the spec; those spec edits remain uncommitted at STOP.
- Last completed checks after those refinements: R3 rational/UTF8/link/table checks, questions check, doccheck check and diff check passed.
- Final report patch applied; final validation/commit command was blocked before execution. Only this lane report is committed for STOP.
- Ignored `build-runs/lifecycle-r3-literals.py` includes those extra distance checks. Final message file5 exists but its commit never ran.
- No merge-queue entry written in this round. Do not treat the report below as clean-tip admission.
- Next only if coordinator explicitly resumes: inspect dirty spec diff, rerun final report validation, commit and decide publication; no automatic fourth research round.
- No original/game/build/portable execution, ABI rows, factory/default or whole-owner claim.

## Research report — 2026-10-23, SAME FINAL R3of3 (publication interrupted)

Published two substantive native proof milestones in
[the owned lifecycle supplement](../specs/automata-controller-lifecycle.md),
§§11.2–11.5, narrowing its §12 ledger and
[AT22's partial answer](../specs/questions/automata.md). Only these public files
and this lane report changed in R3. Existing automata §3.5 already links the
supplement and needed no edit. R1 `77321b20`/`4a4fc554`/`77f5168b`, R2 helper
`4bc47d12`, composition `05adfde4`, publication `3991b3b3` and ignored evidence
remain preserved. R2 was bounded publication, not complete-owner proof.

Ownership was checked before body credit: base primary `00A95D90`, secondary
`00A95D74` and S=C+4, attractor `00A95E48`, generator `00A95F00`. Attractor
29/30 are overrides; generator inherits the newly studied state/geometry entries
but overrides other inputs such as radius/template/activation. The matching PE
SHA256 was remeasured as
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
Export queries came first; primary instruction bytes establish numeric association,
pointer adjustments, widths and branch order. No REA/session action was needed.

Milestone `d1f28698` ADDS Read's actual local chronology: old occupant null-store
then Release before incoming reference primitive; raw field destinations;
unconditional continuation after intermediate status failures; list append without
clear or count increment; final full-EAX inverse of status AL. Helper retention
stays callee-supplied. Same/null/replaced references and partial-output/repeated
Read fixtures are explicit and independently kept alive. It also proves finite
base22 direction midpoints and base23 expanded-rectangle rejection/equality and
height threshold, rather than assuming a circular radius or angle conversion.

Milestone `7e373dd3` ADDS nonnull distance-output composition and directly needed
owned intersection edge-call order, base28–30 selector4 entry/exit/tick branching,
queue lookup versus create request, false/true AL distinctions, full-pointer
nonnull test, tick scale clamp and product-store association; base32's ordered
0x7C-byte row scan, threshold comparisons and local selector4 distance test.
Foreign segment/length/queue-point/manager algorithms remain individual raw calls.
No derived internals or CreateTemplateInstance body were studied. Final review
chose exact 3–4–5 and 5–12–13 coordinates for the distance call fixtures.

Seventeen new R1–R17 fixtures are **PROPOSED / UNEXECUTED**, bringing the corpus
to 46. The earlier L1–L13/N1–N16 remain UNRUN, with L1/L7 still projections.
Fixtures independently supply stream/reference IDs, primitive output identities,
partial mutations and raw return channels; no completed Read/radius/state oracle
is supplied. R14 derives an exact product of 1+2^−24−2^−47 under PC53 nearest:
x's unspilled product/add yields binary32 bits337FFFFE, while z's product spill
then add gives positive zero. This is an analytic discriminator, not an observed
original or portable run. Runtime PC/RC and copied-global initialization are
unmeasured.

Validation passed: `python tools/questions.py check`, `python tools/doccheck.py
check`, `python build-runs/lifecycle-r3-literals.py`, and `git diff --check`.
The R3 helper checks independent exact rational/integer literals, strict UTF8/no
BOM, local file links, table widths and unique ordered L13+N16+R17 fixture IDs.
Call chronologies were audited manually against the cited instructions, not
executed by the analytic helper. AT22 remains one open question; no complete
entry was answered/moved, no new ID was filed and the shared generated question
index was not written. Cargo/clippy/ctest and game/shadow verdicts are n/a.

R3 tooling issues: q.py's Bash service returned Access denied before commands
ran; subsequent read/query work used direct PowerShell. Both proof commits
succeeded despite Git printing packed-refs.lock Permission denied. Shared refs
were untouched. The R1 provider capacity failure and R2 patch mismatch, analytic
denominator correction, question-index heading misread and commit warning remain
preserved below/in ignored history; none is native behavior evidence. No old
admission or R2 qualification was rerun.

Residuals: AT22 remains partial for CreateTemplateInstance, foreign reference
acquisition/stream/segment/length/queue-point semantics, complete callback typing
(state tick's third word and animation tick's trailing two words), global
00B227CC runtime initialization, allocation/exception/reentry/invalid-pointer/
unwind domains, broad FP/trap/payload/overflow and complete derived/factory
lifetime contracts. No zero-position default, standalone base factory, whole
48-entry/6556-byte closure, native wrapper/ABI route or family completion follows.
AT21/AT23/AD3/AD4 and parked glue/caps remain unchanged. No ABI rows were proposed.

This bounded research is useful for review, but STOP interrupted final publication;
the final spec refinements are not a clean committed DOCS tip and are not queued.
No native objects, process/provider/server actions, builds, game
execution, portable engine tests, foreign-worktree/oracle access, branch/ref
changes, merge/rebase/push or subagents were used. This round stops with the
handoff above; no fourth round or automatic continuation is requested.

## Preserved historical report — 2026-10-23, SAME R2of3

This is the prior bounded report, not a new execution/qualification. Its old
residual list and 29-row total are superseded only by the narrow R3 findings above.

Published [the inherited base lifecycle contract](../specs/automata-controller-lifecycle.md)
and integrated its bounded findings in [automata §3.5](../specs/automata.md).
R1 commits `77321b20`, `4a4fc554`, `77f5168b` and all ignored evidence were
preserved. The sole inherited dirty lifetime-field correction was audited and
committed, retaining historical corpus values. Dependency `ef8e5396` was already
qualified in R1; copied R2 admission, 07:48:19 same-model availability exit0 and
07:50:19 host census are retained in `build-runs/evidence-run4/`. No process,
server, native/build/game, branch/ref or foreign-worktree actions were taken.

R1 established C versus S=C+4, QI/refcount and template/occupant ownership,
Shutdown versus direct destruction, Init/activation local chronology, raw-byte
flags, scaled float tick/expiry equality, finite bounds/grid storage, first-match
removal and concrete dispatch limits. The serializable base identity 4A18672B
does not establish a registered base factory. The copied raw global 00B227CC
still has unknown runtime initialization. R1's previously measured PE SHA256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.

R2 helper proof `4bc47d12` derives primary-vtable `00A95D90` entries and byte
paths for PlaySFX `00515770`, CheckEffectTriggers `00516B40` and HandleViewChanged
`00515870`. PlaySFX stores the selected ID at C+0x3C before start/position
commands, ignores their returns and does not clear it on stop. Effect predicates
short-circuit through QI output Q, while the final Release uses source R; false
QI with nonnull output performs no cleanup. View handles null services, forced
visibility, repeated position/bounds/height outputs, ordered five-point tests and
the final flag setter; normal return is true even when visibility is false.
Foreign services remain single typed calls with raw outputs/returns/ref effects.
No foreign service internals or derived selection/emission/motion algorithms
were used to close these helpers. Export/asm/decomp navigation sufficed; no REA
call or session operation was needed.

Composition milestone `05adfde4` adds N1–N16 proposed, **UNEXECUTED** rows to
the original thirteen: 29 total. Init can activate, deactivate on a failed
effect condition, then still start sound. Raw request2 can clear activation yet
notify/start and enable delay. Repeated Init/Shutdown, equal/null clock swaps,
source/output identity separation, safe empty-list true-with-null Q, selected
ID0, forced/unavailable view and ordered point coordinates are explicit.
L1/L7 retain projections because inputs are unspecified; L10 now derives its
outputs through owned bounds/lot helpers from one occupant output operation.
The source/reference/tick/signature audit retains the true earlier conclusions.

Validation completed:

- `python tools/questions.py index`, then `check`: passed; regenerated index
  has no net diff, no accidental new ID and no entry moved to answered.
- `python tools/doccheck.py check`: passed.
- `python build-runs/lifecycle-r2-literals.py`: passed. Exact rational tick
  product is 1073741875/1073741824; PC24/53/64 nearest-even stores delta1;
  upward rounding can store the next binary32. Runtime PC/RC was not measured.
  Numeric subsets for L10/N1/N2/N6/N9/N15 were checked independently; controller
  call chronologies were manually audited from primary bytes, not executed.
  L1–L13/N1–N16 IDs are unique/ordered, relevant tables align and local
  links in edited sections and this report resolve.
- `git diff --check`: passed; only four public documentation paths changed
  across the lane. Cargo, clippy and ctest: n/a for this docs-only RE lane.
  Game labels/shadow verdicts: none; no native objects, Init, sound or render
  execution and no portable implementation/golden outputs.

Tool issues remain distinct from behavior evidence. R1 provider capacity failure
and command/patch errors remain preserved. R2 corrected an expected-line patch
mismatch, the analytic helper's own mistaken expected rational denominator, and
a bold R2 heading initially misindexed as question R2. Corrected checks passed;
none was a native behavior failure. Commits succeeded despite Git also printing
packed-refs.lock permission denied; shared refs were left untouched.

Questions: AT22 has further partial source findings in
[questions/automata.md](../specs/questions/automata.md), and stays open. Residuals
are owned state/behavior/template-instance/intersection helpers, complete callback
signatures, Read ownership, runtime position initialization, allocation/exception/
reentry/invalid-pointer and broad FP/overflow domains, plus complete derived
contracts. AT21/AT23/AD3/AD4, parked glue/families and old warnings are unchanged.
No ABI rows proposed, C ABI changes, 48-entry/6556-byte coverage credit, whole-owner
completion, native route, default enablement or family admission is claimed.

Ready means this coherent bounded DOCS publication can be reviewed and landed.
R2 admission says the original independent brief peer was already satisfied;
the coordinator independently reviews the queued exact committed tip. No extra
worker peer or approval prerequisite was invented. Continue the named AT22
residuals only under a separately authorized scope; no automatic further round.
