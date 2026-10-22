| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotmodels-contracts4: LM1–LM8 native contracts | Codex / `re-lotmodels-contracts4` (RE) | blocked: coordinator STOP; LM3/LM5/LM6 answered, LM1/LM2/LM4/LM7/LM8 remain; no runtime claims | Preserve findings; coordinator review required before any further RE. |

## Handoff

- Goal: resolve only LM1–LM8 from primary native evidence; no engine reads/edits.
- Worktree reserved, detached HEAD reused; no ref changes.
- Read lot-model spec target sections, open questions, lane/merge format and short handoff.
- Done: `81d9d85b` binding-failure and cell corner/placement contract.
- Added foundation buffer order, exact step arithmetic, FSH groups and 11 independent vectors; LM3/LM5/LM6 answered.
- Native proof: missing zero key disables foundation/wall binding; fallback only after nonzero failure.
- Cell x/z unsigned bytes, four-record strip and full orientation table verified in instructions/data.
- Done: `dd213dcc` answered LM3/LM5/LM6 and added 11 native-derived independent worked vectors; question index regenerated.
- Additional uncommitted spec edits now preserved: basic zero-initialized 16-bit refcount/owner lifetime, destructor release order, bounded 51 named-entry inventory, wall facing flag and neighbour suppression.
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
- Last successful guard measurement: 115941/150000. Guard subsequently denied publication patch: PARKED, user directed STOP RE; no further rounds authorized.
- Next: coordinator reviews committed bounded findings and known citation/summary corrections; no autonomous continuation authorized.
- q.py cannot launch Bash (E_ACCESSDENIED); use bounded PowerShell output and re_query directly.
- No game runs, ABI changes or runtime claims planned.
