tag: overlays-shadow5-r1-2032b-370
run: 370
date: 2026-10-24
lane: glue-overlays-shadow5
dll: FreeCity4-glue-overlays-shadow5; sha256 prefix b4df96ae
pacing: turbo
verdict: clean
switches: overlays_shadow=1, observe=1, popup_dialogs=0; Konradshohe; end_date=2032-01-01; implementation bd552f9e9 plus timing fix 58f2d0907
result: Clean at 2032-01-09, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; 57.907 simulated-run real seconds, 77 s harness wall, slot 2. Exact log: overlays contract shadow: 73/73 lines exact mismatches=0 fields=7300 scope=sentinels records=blocked(OV12). Load plus 72 monthly samples each report 100/100 mismatches=0 managers=4. All harness problem counts zero. This verifies only the documented no-handle/direct-placement getter contracts; live overlay records/cells/textures remain blocked by OV12. Request and evidence: build-runs/game-queue/done/overlays-shadow5-r1-2032b.req and build-runs/game-results/overlays-shadow5-r1-2032b.{verdict,log}.
