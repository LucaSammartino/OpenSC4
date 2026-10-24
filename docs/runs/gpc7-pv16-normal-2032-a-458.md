tag: gpc7-pv16-normal-2032-a-458
run: 458
date: 2026-10-24
lane: glue-propdev-close7
dll: 489fe5e3 (source e5edd7566)
pacing: normal
verdict: clean
switches: enable=simulator; propdev_shadow=1; sim_turbo=0; native propdev; Konradshohe to 2032-01-01
result: clean through 2032-01-01, foreign_exceptions=0, ref_warnings=0, rtc=0, 146.891s. Exact coverage line: propdev 0/72; zero resyncs. All 24197/24197 selector events matched, 45465 complete streams, zero invalid streams. 2932 aggregate mismatches/incomplete operations; creation/dated effects deliberately remain ineligible. Night callback count 0: the fixture did not exercise PV16 and this run provides no night-capture exactness claim. Evidence: build-runs/game-results/gpc7-pv16-normal-2032-a.{verdict,log}; wait session 30269 collected to terminal. Later requester-tag source changes are not represented by this DLL.
summary: native propdev safe to 2032, selectors exact, no night callback exercise
