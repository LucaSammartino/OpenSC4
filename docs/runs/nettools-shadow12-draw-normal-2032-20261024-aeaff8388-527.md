tag: nettools-shadow12-draw-normal-2032-20261024-aeaff8388-527
run: 527
date: 2026-10-24
lane: glue-nettools-shadow12
dll: FreeCity4-glue-nettools-shadow12; source aeaff8388; sha256 prefix 76cfa9bc
pacing: normal
verdict: clean
switches: enable=simulator; disable=nettool,netmgr; nettool_drag_shadow=1; nettool_wide_drag_shadow=1; Konradshohe to 2032-01-01; turbo=0
result: clean, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0; date=2032-01-01, real_seconds=110.390. nettool wide drag shadow 73/73 (48 vectors/line, mismatches=0, unavailable=0). nettool drag shadow 0/73 (160 vectors/line, mismatches=6, unavailable=0): same pipe reverse-diagonal differences as run 512, filed NG8. This is not a turbo-only difference. Scope is public draw projection only; no whole-owner or placement credit. Logs and verdict: build-runs/game-results/nettools-shadow12-draw-normal-2032-20261024-aeaff8388.*.
