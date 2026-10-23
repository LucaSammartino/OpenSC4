| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sprites-contracts4: animated-sprite native contracts | `re-sprites-contracts4` (RE docs lane, detached round 2) | in progress: bounded AS6/AS7 evidence and retained-proof reconciliation | Verify rotation and quad instructions; preserve explicit residuals. |

## Handoff

- Goal: bounded AS6/AS7 publication and retained-proof reconciliation. AS2 excluded; AS1 provenance remains open.
- Done: this milestone adds instruction-derived AS7 §§5–6 geometry/store/box/cache/draw/pick contracts and independent proposed UNEXECUTED R2-Q1/Q2/Q3 vectors.
- Done: c588e64e is the AS7 milestone. This next milestone adds AS6 axis/order/sentinel/LookAt proof and independent proposed UNEXECUTED R2-R1/R2-R2/R2-R3/R2-D1 vectors; fixes clock/LookAt/citation contradictions. Partial-mask last-bit rounding remains qualified, not a portable oracle.
- Identity: bin/SimCity4.exe SHA256 3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b matches export-summary.txt, base 00400000.
- Evidence: UpdateView 007FE400–007FEA84, Draw 007FEA90–007FED4C, Pick 007FD710–007FD7C6; GetViewVolume 007FF340–007FF3BE, queried export/bytes before supporting decomp.
- Next: verify SetOrientation mask/composition 007FC4B0–007FCC69; supporting decomp _unnamed/region_007f0000.c:2350; prove noncommuting and sentinel fixtures, reconcile LookAt target/forward.
- Before ready: reconcile §1.1 citation, defaults/lifetime/interface/inventory, clock duplicate prose, and partial questions. No game/native execution, ABI proposals or coverage credit.
- Queue: task-specific prohibition overrides generic finish queue instruction; no queue file will be written.

## Retained R1 report (historical, not readiness)

- Round 2 resumed 2026-10-23 under run4-re-sprites-contracts4-r2-manifest.json; historical STOP below is superseded only by the copied host admission.
- Authority: build-runs/coordinator-sprites-r2-host-proof.json records full admission exit 0, dependency ancestor exit 0, role RE, expected round 2, unparked; runtime guard remains mandatory.
- Current next: export/asm proof for SetOrientation, LookAt and UpdateView; reconcile contradictions before any ready publication. Historical findings below remain evidence, not readiness claims.

- Goal: AS1/AS3–AS9 only, cS3DModelInstanceAnimatedSprite; no engine reads/edits occurred.
- STOP: guard denied further work: "User directed STOP RE: prioritize verified/replaced; preserve findings, commit handoff and return blocked. No further RE rounds authorized."
- Done: 69ee90df native instruction proof for clock/reversal/returns, sequence selection/frame quirks, fade/highlight/self-lit.
- Done: d45820c2 native load rollback, key/IID/ownership and orientation-record/atlas consumer contracts, independent worked vectors.
- Preserved draft: owner defaults, shared-interface table, lifetime and bounds/sort getters added after d45820c2; not reconciled/reviewed before STOP.
- IMPORTANT: draft §1.1 contains a literal unfinished citation `0x00801CF0–007?`; its following exact address range is the intended evidence. Do not publish this draft as ready.
- Existing §3.1 and §5 still need reconciliation: LookAt first vector is target; Draw skips its boolean highlight-pass argument, not the stored highlight flag; fade threshold is alpha×0.5, not zero.
- AS8 measured: true/−100 no-op; false always disable/alpha=1; true/−200 disables; otherwise <=binary32 0.99 accepts negatives/NaN, >0.99 disables; all return 2.
- AS4 measured: failed acquisition rolls back only this call's successful prefix; five acquisitions plus non-null binding required for true. Frame count/scales are not rolled back.
- AS5 measured: SetRate literal store (−100 resolves to literal −200 in Tick); subtract direction AFTER reversal; AdvanceFrame always 2; skip loop has no progress guard. SetFrame uses remainder only for n>length.
- AS9 measured: GetBounds copies local box; WorldBounds adds position only; sort writes binding.table[metric page] or zero; GetResModel null; getters do not recompute.
- Residual AS1: controller provenance beyond constructor/load (both leave +0x34 null), complete data-source reconciliation. AS3: exact 60-function/10482-byte accounting and remaining transform-helper details.
- Residual AS6: noncommuting Euler composition/−100 rule. AS7: exact UpdateView quad/order/bounds math and numeric instruction verification; draw/cache/pick observations below are not yet incorporated into spec.
- Draw proof: 0x007FEA90 skips nonzero boolean argument or signed context zoom<2, returns 1; zoom/rotation/view-dirty refresh frame, but UpdateView only when view-dirty; camera/mpp changes alone do not invalidate.
- Draw tail 0x007FEC69–007FED4C: model transform; model/self-lit shade using RGB; packed state; if fade, transparency(alpha), alpha-func(4,float(alpha×double 0.5)); texture(binding.table[page],0); DrawPrims(2,10,4,vertex block); ResetShade then ResetTransparency. No render-state restore/failure branch.
- Pick 0x007FD710–007FD7C6 tests cached translated vertex triangles (0,1,2), then (2,3,0); no view refresh. Thunks subtract [this−4], not fixed 4.
- No questions moved/answered or question index regenerated before STOP. AS2 untouched. No ABI proposals, game runs, gates, implementation/runtime/default/coverage credit, or merge queue.
- Next: none authorized; coordinator must release this parked worktree before reconciliation. Do not restart another RE round.
- Native sources: C:/Users/tlori/FreeCity4/decomp/cS3DModelInstanceAnimatedSprite.c and tools/re_query.py in main checkout.
- q.py cannot start Bash (E_ACCESSDENIED); use local PowerShell commands with capped output.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-sprites-contracts4-r1-manifest.json.
