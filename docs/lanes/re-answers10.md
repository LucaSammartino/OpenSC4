| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-answers10 | `af850af1f5c7bf5b5` / `re-answers10` | **merged** (group 1 `622086f`; the rest `a68f423`, specs only, no ABI rows): TG1-TG22, ZG18-ZG21, NG35-NG44, AQ11-AQ13, FL10-FL13, PW9-PW11, FS15-FS17, DG28, DG31 per-day note, RN1, CM4-CM9, IN2, CB6-CB14, PK4-PK9. **DIFFERS:** CK14, HW6, HW7, HW12, TG1, TG7, TG18, TG21, ZG18, ZG21, NG35, NG36, NG37, NG43, NG44, FL12, FS15, FS16, DG28 (city Shutdown releases the BDS before the occupant manager), CM5 (null map: no seed, no passes), CB6 (civic Read reverses each multi-entry chain; a repeated key keeps the last value), CB7 (Release destroys without Shutdown) | done. Apply: TG answers in traffic phase 2; ZG/NG/FL/FS in their cores; cores10b fixes CB6 (civic §12 tests 9-10), the repeated-key rule and CM5. The glue save sweep confirms the CB6 corpus expectation (re-write = chains reversed, second round trip identical). Open: DG28 (why our growth developer holds the last reference), DG31 census CPU time, FS17 count not cross-checked, CB8 automata controller with a null target |

## Retired roster row

Kept verbatim from the HANDOFF §0 `| Lane | Agent ID | Worktree | Reserved | State |` table (2026-10-14 wind-down); the generator does not read this section.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| RE answers 10: CK7-CK15, HW5-HW12 (first), TG1-TG22, ZG18-ZG21, NG35-NG44, AQ11-AQ13, FL10-FL13, PW9-PW11, FS15-FS17, DG28, DG31 per-day amounts, RN1, cores10b questions | `af850af1f5c7bf5b5` | `re-answers10` | - | merged `622086f`, `a68f423` |
