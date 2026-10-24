# landvalue: open questions
<!-- Questions about [landvalue.md](../landvalue.md). Append new entries at the END; answered entries move to answered/landvalue.md. -->

<!-- from: coordinator train65 (2026-10-22) -->
- **LV2 - verification of LV1 per-Step capture:** train65 has71/72 exact, versus train64 72/72. Pass43 at2029-07-07 has5 intrinsic,2 total and1 slope cell differences, mean altitude270.05667/270.05804, settled=0. An earlier SimNewDay snapshot reproduces native intrinsic/mode/mean but not total. Does capturing inputs at each original Step read plus the per-developer loaded previous map (LV1) restore this pass? Preserve the trace and compare the actual read-boundary inputs; do not adjust algorithms, sampling or expected outputs. See [incident](../../incidents/train65-landvalue-timing.md).

<!-- from: verify-landvalue-step4 (2026-10-22), additional runtime evidence for LV2 -->
LV2 evidence: `lv4r2-sample-20261022-0941-turbo` reached 2032-01-20 clean/ref0, 73/74 output-exact comparisons. Pass16 at2027-04-06: intrinsic5,total2,slope0,type0,mean270.05667/270.05804, mismatch count8; previous-D chosen (live total262). Day1/31 reproduces intrinsic/mode/mean and total. `lv4r2-sample-20261022-0944-normal` reached2032-01-01 clean/ref0,70/72 output-exact. Pass3 at2026-03-02: intrinsic1,total1,slope0,type0,scalars0, mismatch count2; previous-D chosen (live total647). Day1/28 reproduces intrinsic/mode/mean and total. These are pass-end samples, with native_step_capture=0 throughout, not per-Step proof. Both use DLL SHA256439fd26fcb2b1abb43f1aa7e4ac437b082917c450a0a549404bdeb016956a7ad. Logs, verdicts and parsed records remain under this worktree's `build-runs/game-results/`; no algorithm/math/golden/sampling adjustment was made.

<!-- from: verify-landvalue-step4 (2026-10-22) -->
- **LV7 - section 6.1 loaded startup previous-map verification:** normal-speed run `lv4r2-sample-20261022-0944-normal`, pass1 at2026-01-28, differs in391 total tracts (max delta42), with intrinsic/slope/types/scalars exact, average72/72, mode272/272, mean270.05533/270.05533, settled0. Only live D was available; no previous-map rerun occurred. Is this startup discrepancy resolved by the section6.1 restored-map snapshot plus the actual per-developer state7 capture once LV6's ABI prerequisite is recorded? Preserve this trace and compare those actual maps; the current sample cannot establish the causal map or justify changing the algorithm. [Unresolved runtime verification gap; zero replacement/input-fidelity credit.]

<!-- from: glue-traffic-flake6 (2026-10-24), train166 first-difference audit -->
LV6/LV2 evidence: `regress-train166-shadows` first land-value mismatch is pass5,
2026-05-08: intrinsic2, total1, slope0, types0, mean270.05533/270.05667,
four mismatches overall. No daily snapshot among30 reproduces it (closest day1,
one intrinsic tract differs). The first traffic mismatch occurs much later,
2027-06-13, so this land-value failure cannot be caused by that traffic-shadow
drift. This is consistent with §6.1 inputs changing within a native pass;
the pass-end sample is insufficient. LV6's safe installation/lifetime contract
still blocks implementing faithful per-delivery capture. No formula, comparison,
sample selection or acceptance criterion was changed.

<!-- from: glue-traffic-flake6 (2026-10-24), third house repeat -->
LV6/LV2 repeat evidence: `traffic-flake6-637a-house-t3-r2` is clean/ref0 with
traffic72/72 exact and zero traffic resyncs, but landvalue71/72 exact. Its sole
landvalue difference is pass47 at2029-11-08: intrinsic5 (max delta2), total2
(max delta2), slope0, types0, mean270.05667/270.05804, eight mismatches overall.
The log labels input_capture=pass-end-sample/native_step_capture=0; its day1/32
diagnostic reproduces intrinsic/mode/mean but total still differs in428 tracts.
This preserves the LV6 input-boundary gap despite three exact traffic repeats;
it does not establish a turbo-only cause. No implementation, sampling, formula
or acceptance adjustment was made; landvalue stays blocked on LV6.

<!-- from: glue-landvalue-step6 (2026-10-24), measured LV2 evidence -->
LV2 evidence: `lvstep6-20261024-isolated-t1` reached2032-01-10 clean/ref0 with72/73 exact output comparisons (isolated landvalue, turbo; not house). Pass2 at2026-02-06 has intrinsic6/total3/slope0/types0 differences, mean270.05667/270.05804, mismatches10. No SimNewDay sample among9 reproduces native inputs; closest day1 still differs in3 intrinsic tracts. SHA256863F529E12DEC6F3B196B1F1B39DA1EF6CFDEB03ED517360E79FC4D322B7EC55, run388. Native capture remains0 per sections6.1.2-3/LV8; no algorithm, sampling or acceptance adjustment. The two prior multi-shadow requests never loaded a city and contribute no output/fidelity evidence.

Normal-speed counterpart: `lvstep6-20261024-isolated-normal`, same DLL, reached2032-01-01 clean/ref0,73/73 comparisons with mismatches=0 (run389, real110.469s). This fulfills the normal-speed check for this turbo difference without claiming that all historical LV2/LV7 failures are turbo-only. Native capture remains0 and no acceptance or algorithm change was made.

<!-- from: glue-landvalue-capture7 (2026-10-24) -->
- **LV9 - host provision needed after LV8 native closure:** Section 6.1.4 explicitly requires an identified host admission/exclusion mechanism and supplies no existing game installation API. The plugin currently has lifecycle notifications, QI and post-pass observers, which LV8 rules out as exclusion proof. Please identify or coordinate a host-side provision that covers pre-fetch admission on every affected thread, both city startup paths and input lifetime through Shutdown, or confirm that native capture remains ineligible until such a host exists. This is an integration dependency, not a request to re-infer native fields/signatures or reinterpret lifecycle ordering. Vectors 26-31 now have a deterministic reference-host fixture, but it does not install that host in the game. Native capture remains disabled; LV2/LV7 remain open and sampled output is measured separately. [No supported host mechanism identified; input_capture=pass-end-sample; native_step_capture=0.]

<!-- from: glue-landvalue-capture7 (2026-10-24), preserved LV2/LV7 measured fallback -->
LV2/LV7 evidence: `lvcapture7-20261024-isolated-t1` reached2032-01-02 clean/ref0,71/73 exact (isolated, turbo; not house). Pass1 at2026-01-28 has383 total differences(max15), intrinsic/slope/types/scalars exact; only live developer map available (LV7). Pass3 at2026-03-06 has1 slope difference(max0.013541221618652344), all other fields exact, previous developer map selected (LV2). SHA256ABAD0198BAA9D88E545B6FC76BAD3EF57350671A28F928F2A14C95FB62C18B59, run410. Capture remains pass-end-sample/native_step_capture=0 under LV8; no formula/sampling/golden/acceptance change. Normal-speed counterpart requested; no causal claim from this sample.
