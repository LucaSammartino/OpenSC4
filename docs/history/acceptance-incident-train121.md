# Train121 LandValue worsening

Train121 landed at `373e2400bacb5aaf97f1f7fa283724c175637ad4` on 2026-10-23.
The coordinator missed a shadow worsening that the regression output explicitly reported:
LandValue went from 72/72 exact comparisons to 71/72. The subsequent unchanged coverage
resolver correctly withdrew 27 functions and 6,711 verified bytes, reducing verified
coverage from 19.7% to 19.6%. That decrease remains recorded. The clean game-process
verdict and zero lifetime warnings do not establish shadow equivalence.

At pass67, 2031-07-06, the comparison reports four differing intrinsic cells, two total
cells and one differing mean-altitude scalar. The preceding and following comparisons
are exact. An existing diagnostic earlier-day snapshot reproduces intrinsic, mode and
mean bitwise, while its total differs in 352 cells; the pass-end candidate differs in
only two total cells. This supports a temporal input-coherence gap without proving its
complete cause. It does not justify changing arithmetic, selecting a convenient sample,
discarding the failure, restoring credit or rerunning solely to seek a clean result.

The five LandValue production/CAPI/plugin files and the public specification/layouts
are unchanged between train120 and train121. The two game sessions are not an identical
input replay: native worlds have already diverged before the bad pass. Neither unchanged
source nor different session seeds establish the cause. The actual disclosure remains
`input_capture=pass-end-sample; native_step_capture=0`.

The ABI progress layouts are already published. Historical text saying they are absent
is stale. The remaining public contract gap is the safe native observer installation,
exclusion from in-flight callbacks, new-city creation timing, lifetime and restoration
boundary described in [landvalue.md](../specs/landvalue.md), section6.1.2. No complete
native capture adapter or source correction is established by this investigation.

Local retained evidence:

- `build-runs/regress-train121.out`: LandValue summary and explicit `1 worse` delta.
- `build-runs/coordinator-train121-landvalue-evidence.json`: raw-log SHA256 pins,
  comparisons, control requests and unchanged Git blob identities.
- `build-runs/coordinator-train121-landvalue-diagnosis.md`: independent detailed review.

The acceptance error is concrete: `tools/regress.ps1` reports process cleanliness as
PASS while reporting worsened comparisons separately. The coordinator read the former
without acting on the latter. Until a qualified source change adds structured review
enforcement, every train acceptance must inspect the complete comparison delta and
record each worsening explicitly before landing. A process PASS alone is insufficient.
An unresolved worsening must remain held for investigation; a documented investigation
must retain the actual failed comparison and resulting coverage. No gate record may be
edited to manufacture acceptance.

For this already-landed train, the failure and lower coverage are preserved. Subsequent
work must address the demonstrated capture prerequisite through the normal guard,
original family budget and independent review. No fourth round or renamed capped lane
is authorized by this incident.
