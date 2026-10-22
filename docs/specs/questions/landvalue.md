# landvalue: open questions
<!-- Questions about [landvalue.md](../landvalue.md). Append new entries at the END; answered entries move to answered/landvalue.md. -->

<!-- from: coordinator train65 (2026-10-22) -->
- **LV2 - verification of LV1 per-Step capture:** train65 has71/72 exact, versus train64 72/72. Pass43 at2029-07-07 has5 intrinsic,2 total and1 slope cell differences, mean altitude270.05667/270.05804, settled=0. An earlier SimNewDay snapshot reproduces native intrinsic/mode/mean but not total. Does capturing inputs at each original Step read plus the per-developer loaded previous map (LV1) restore this pass? Preserve the trace and compare the actual read-boundary inputs; do not adjust algorithms, sampling or expected outputs. See [incident](../../incidents/train65-landvalue-timing.md).

<!-- from: verify-landvalue-step4 (2026-10-22) -->
- **LV6 - section 6.1 native Step capture ABI record:** please record the already specified native progress layouts in `docs/abi/layouts.md`: the land-value interface-relative pipeline state, both iterator pointers, ignoreZoning, developer cursor/header, iterator fields and developer map node fields, including their widths and the canonical interface base. The message-target slot 3 / `DoMessage` ABI is present in `interfaces.tsv` and section 11.3, but these field layouts are absent from `docs/abi`. CONTRIBUTING requires game layout dependencies to be recorded there before engine glue uses them. Is a version-gated, QueryInterface-obtained message-target slot-3 forwarder, with progress read through the separately obtained land-value interface before/after each native SimIdle/SimNewDay delivery, the supported boundary? Please document any additional ABI needed for that adapter. [Stop the native-layout/hook path until recorded; retain and explicitly label the existing pass-end sample. No input-boundary fidelity credit, even for zero output differences. No algorithm, math, golden or sampling adjustment.]

<!-- from: re-landvalue-step-abi4 (2026-10-22), partial static LV6 evidence -->
LV6 static follow-up (2026-10-22, re-landvalue-step-abi4): §6.1.1 and §11.3 now establish
canonical L/M bases, synchronous slot-3 before/forward/after ordering, reference ownership,
iterator semantics and per-developer capture. Exact-width layout proposal:
`build-runs/pending-abi/re-landvalue-step-abi4-layouts.md` (coordinator must review/apply to
`docs/abi/layouts.md` and run the full train gate; no direct ABI edit or gate verdict here).
**LV6 remains open:** no installation path has been proved that intercepts existing native agent
deliveries while preserving identity/order, shutdown removal, notification lifetime and early
loaded-map capture. QueryInterface alone installs nothing. Until ABI application and that proof,
retain `input_capture=pass-end-sample; native_step_capture=0`; no input-fidelity credit.

<!-- from: verify-landvalue-step4 (2026-10-22), additional runtime evidence for LV2 -->
LV2 evidence: `lv4r2-sample-20261022-0941-turbo` reached 2032-01-20 clean/ref0, 73/74 output-exact comparisons. Pass16 at2027-04-06: intrinsic5,total2,slope0,type0,mean270.05667/270.05804, mismatch count8; previous-D chosen (live total262). Day1/31 reproduces intrinsic/mode/mean and total. `lv4r2-sample-20261022-0944-normal` reached2032-01-01 clean/ref0,70/72 output-exact. Pass3 at2026-03-02: intrinsic1,total1,slope0,type0,scalars0, mismatch count2; previous-D chosen (live total647). Day1/28 reproduces intrinsic/mode/mean and total. These are pass-end samples, with native_step_capture=0 throughout, not per-Step proof. Both use DLL SHA256439fd26fcb2b1abb43f1aa7e4ac437b082917c450a0a549404bdeb016956a7ad. Logs, verdicts and parsed records remain under this worktree's `build-runs/game-results/`; no algorithm/math/golden/sampling adjustment was made.

<!-- from: verify-landvalue-step4 (2026-10-22) -->
- **LV7 - section 6.1 loaded startup previous-map verification:** normal-speed run `lv4r2-sample-20261022-0944-normal`, pass1 at2026-01-28, differs in391 total tracts (max delta42), with intrinsic/slope/types/scalars exact, average72/72, mode272/272, mean270.05533/270.05533, settled0. Only live D was available; no previous-map rerun occurred. Is this startup discrepancy resolved by the section6.1 restored-map snapshot plus the actual per-developer state7 capture once LV6's ABI prerequisite is recorded? Preserve this trace and compare those actual maps; the current sample cannot establish the causal map or justify changing the algorithm. [Unresolved runtime verification gap; zero replacement/input-fidelity credit.]
