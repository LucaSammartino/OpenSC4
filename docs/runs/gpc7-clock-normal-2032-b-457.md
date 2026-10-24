tag: gpc7-clock-normal-2032-b-457
run: 457
date: 2026-10-24
lane: glue-propdev-close7
dll: 80210e7d (source fd5860eae)
pacing: normal
verdict: clean
switches: enable=simulator; propdev_shadow=1; sim_turbo=0; script=build-runs/propdev-night-actions.txt (clockmode 0 on 2026-01-28); native propdev; Konradshohe to 2032-01-01
result: clean to 2032-01-01, foreign_exceptions=0, ref_warnings=0, rtc=0, 132.593s. Exact coverage line: propdev 0/72; zero resyncs. Selector events 23581/23581, complete streams 45465/45465, zero invalid streams; final creation requester-tag projections 588/588 matched. 2966 aggregate mismatches/incomplete operations remain explicitly ineligible due to creation/dated effects (PV18). Night callbacks/draws both 0 despite clock script succeeding: native night capture still unexercised. No whole-propdev verification credit. Corrected all gate qbuild-20261024-065026-build-ctest PASS: Rust 6939/0, clippy clean, CTest 78/78. Wait session 46972 / collector cell 74 collected to terminal; no game request outstanding. Full evidence in build-runs/game-results; analyze-propdev.py aggregates the single dated monthly propdev lines.
summary: native propdev safe, selector streams and requester-tag projections exact, full verification open
