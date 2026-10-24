tag: split11-r2-combined-retry-628
run: 628
date: 2026-10-24
lane: tool-regress-split11
dll: coordinator integration snapshot SHA256 4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1
pacing: turbo to 2032-01-01, ShadowSplit 1, queued only after train200 cleared
verdict: FAIL (parent regression); native queue verdict clean, repeated watchdog trips
switches: unchanged house shadows and standard replacements
result: split11-r2-c1-retry reached 2032-01-02, real_seconds=815.922, days_per_second=2.65, watchdog_stalls=2, foreign_exceptions=0, ref_warnings=0. Queue wall 832 s; regress reported wall 846 s; complete Python invocation 971.09 s. Replacements reached 2032-01-01 clean. coverage.read_regress reports pass=False, 52/52 shadow labels exact; flammability 54/54, propmgr 220/220. Repeated watchdog trips prevent a passing regression despite exact shadow comparisons. The legacy reported wall is sampled before summary/delta processing (regress.ps1 line 299); invocation wall includes that work. Exact per-label comparison is recorded in the lane report after the split retry; logs remain build-runs/regress-split11-r2-c1-retry-*.
