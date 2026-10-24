| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-ordinance-shadow6: record-only ordinance monthly shadow | Codex, detached `glue-ordinance-shadow6` worktree | ready: full gate PASS; ordinance 72/72 exact to 2032; no C export changes | Coordinator land, credit coverage, append ordinance_shadow to the existing regress shadows profile on current main. |

## Delivered

`ordinance_shadow = 1` (environment `FC4_ORDINANCE_SHADOW`) enables a default-off, record-only shadow. It requires FreeCity4's simulator and the original ordinance simulator. The wrapper delivers the original month message exactly once, including capture-failure paths.

Before delivery, `WriteCapture` copies the original simulator and each standard ordinance through their serialization interfaces (ordinance sections 9.1-9.3). The core reads these copies, including pending charges and raw saved flags. References resolve exclusively to private core handles. The host retains interface-based city reads; object-management, messaging and city mutation callbacks are removed. Copies live only on the delivery thread and are destroyed before their acquired original references are released.

After delivery, comparisons cover available/on/enabled state, adjusted and current monthly income, conditions, simulator state getters, monthly income/expense totals and cumulative effects. Effect properties include the documented section 6.2/6.3 queries and live misc holder enumeration, at indices 0..8. Float factors are compared by bits; offsets use the existing compiled 32-bit compatibility interface. One dated line reports each month, with bounded separate mismatch diagnostics.

The native test seam exercises null/empty snapshots and snapshots with live flags and a nonzero adjusted amount, checking that original flags, money and reference counts remain unchanged. No C ABI exports or version changed.

## Validation

- Final stable-source gate: `qbuild.py --wt <WT> all`, **PASS**. Rust workspace **6876 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**; native build **47/47**; full CTest **72/72**, native verify 79.2 s. Queue request `qbuild-20261024-023929-build-ctest`.
- The gate's tree-change note came from a documentation-only handoff commit during its wait. Engine source stayed stable.
- Focused return-channel check: **0 unreviewed uses, 76 reviewed**.
- Earlier failed evidence retained: `build-runs/gate1-fc4verify-build.log` (incorrect date member, fixed); `build-runs/gate2-fc4verify-build.log` and `gate2-fc4verify-ctest.log` (71/72; missing section 4.2 bool-return citations, fixed). Each retry followed that specific correction.
- Real city: **`ordshadow6-20261024-turbo-a 72/72`**, every monthly line exact from 2026-02-01 through **2032-01-01**. **20 records, 43 effect properties, 956 checks/month; 68832 checks total.** Verdict **clean**, **ref_warnings=0**, foreign exceptions/stalls/RTC/overruns all zero. Runtime 83.437 s, harness wall 96 s.
- [Run 374](../runs/ordshadow6-20261024-turbo-a-374.md) records the request, DLL SHA256 and evidence paths. Final line: `ordinance shadow 2032-01-01: records=20 checks=956 effects=43 captured=1 mismatches=0 months=72 clean=72`.

## Scope and questions

No spec questions were needed. The measured scope is the 20 shipped ordinance classes; unknown foreign classes reject capture rather than invoke their mutators. Mod effect-array indices beyond 8 are not measured. Each month is reseeded from original records, so this measures the monthly transition with identical inputs rather than persistence across independent core months. Normal-speed rerun was unnecessary: turbo was exact.

## Coordinator follow-up

Land this detached HEAD and credit the measured ordinance shadow in the coordinator-owned coverage tables. Append `ordinance_shadow` to the existing single regress `shadows = ...` string after updating main. This lane did not inspect or edit `tools/`, merge main, or alter shared refs, as required by its clean-room and detached-worktree instructions. No build or game request remains outstanding.
