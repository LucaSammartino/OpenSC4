tag: split11-r2-combined-626
run: 626
date: 2026-10-24
lane: tool-regress-split11
dll: coordinator integration snapshot SHA256 4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1
pacing: turbo to 2032-01-01, ShadowSplit 1, queue after coordinator cleared
verdict: FAIL (stalled, incomplete)
switches: unchanged single house shadows profile and standard replacement profile
result: split11-r2-c1 reports regress wall 203 s (Python invocation 210.00 s). Shadows stalled at 2027-02-15, real_seconds 158.734, watchdog_stalls=1; replacements reached 2032 clean. coverage.read_regress found 52 labels, all exact only over this truncated interval: flammability 9/9, traffic 13/13, propmgr 43/43. Not a valid full-horizon speed or parity baseline. Full counts/mismatch sums retained in lane build-runs/split11-r2-measurements.json and regress-split11-r2-c1.out. One fresh combined retry requested; failed evidence retained separately.
