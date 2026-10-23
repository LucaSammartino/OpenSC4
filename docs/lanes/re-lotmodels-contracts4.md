| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotmodels-contracts4: LM1–LM8 native contracts | Codex / `re-lotmodels-contracts4` (RE) | in progress: admitted same-family round 2; LM4 then LM7; prior LM3/LM5/LM6 retained | Prove finite wall branches and cell owned draw tuple; retain precise partials. |

## Handoff

- Goal: resolve only LM1–LM8 from primary native evidence; no engine reads/edits.
- Worktree reserved, detached HEAD reused; no ref changes.
- Read lot-model spec target sections, open questions, lane/merge format and short handoff.
- Done: `81d9d85b` binding-failure and cell corner/placement contract.
- Added foundation buffer order, exact step arithmetic, FSH groups and 11 independent vectors; LM3/LM5/LM6 answered.
- Native proof: missing zero key disables foundation/wall binding; fallback only after nonzero failure.
- Cell x/z unsigned bytes, four-record strip and full orientation table verified in instructions/data.
- Done: `dd213dcc` answered LM3/LM5/LM6 and added 11 native-derived independent worked vectors; question index regenerated.
- Done: `87fc8b73` preserved basic lifetime, 51-entry inventory, wall facing/neighbour draft; initial worktree verified clean. Previous uncommitted wording was stale.
- STOP arrived before final checks; no merge queue, ABI proposal, ABI-table edit, code/test read or game run.
- Residual LM1: RGB/alpha/mode upstream creator and global slot-46 identity; SL initializes global B43D20 via city +13C. Proven updater is LotBaseTextureOccupant::UpdateExistingModels 005DF640.
- Residual LM2: reconcile claimed 82-function inventory, full cell inherited table, slot11 005DEB40, all constructor fields, allocation failures and remaining inherited operations.
- Residual LM4: complete branch-specific seam triangle order/winding/crossing/bounds; proved neighbour-flat construction skip at 005E1D8F–96. Do not substitute a generic wall mesh.
- Residual LM7: full semantic operation/source enums and packed-state fields. Native mode sentinel is 255 (005E0279–84); explicit mode 0 also tints. Current short spec summary still needs that explicit correction.
- Native RGB uses global-light component 0 for every channel, not component-wise lighting (005E0389–005E03D4). Mode3 capability fallback halves unsigned alpha before normalization; flat vertex/delegated alpha retain original. Force8 follows pass filtering, so cannot bypass return2.
- Combiner 00B0A878 after Init: bytes 0..11 = 00 00 02 21 01 00 03 01 00 00 00 00; draw changes byte0. Driver SetCombiner 00884A70 decodes RGB op byte0, alpha op byte4 and argument triples 1/2/3,5/6/7; semantic enums not reconciled.
- Residual LM8: full unordered/overflow outcomes and invalid side/rotation domains. Cell x/z unsigned-byte multiplication cannot overflow; arbitrary foundation/wall zoom arrays lack owner checks.
- Correction pending in §4.1: flat DrawPrims citation must be 005E0424–005E0433, not 005E0475–005E0482. No additional spec edit allowed after STOP.
- Wall helper 005E1520: lower endpoint nearby-int(q/16); upper nearby-int(q/16−f32(0.9))+1. Sides0/2 increase x,1/3 increase z. 0.001/0.002 constants thicken perpendicular bounds, not emitted positions (005E1CC0–005E1D08).
- Wall emits only when either terrain endpoint strictly above lot altitude, then skips flat neighbour (005E1D4D–005E1D96). Complete triangle rules remain open.
- Round 2 manifest verified: own lane/re/worktree/round2; copied `build-runs/coordinator-lotmodels-r2-host-proof.json` records full host admission/dependency exit 0. Runtime own state round2/re/not parked; guard within budget.
- Milestone: §3.2 now specifies finite ranges, all three emit branches, skips, UVs, raw-sample bounds and grow-only retained-tail behaviour from primary bytes; pending literal ledgers/review.
- Original executable hash matches export: 3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b. No REA needed.
- Next: independent literal LM4 ledgers; then LM7 packed state and calls. No broader owner/exception audit.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell output and re_query directly.
- No game runs, ABI changes or runtime claims planned.
