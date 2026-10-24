tag: overlays-shadow5-r1-2032a-369
run: 369
date: 2026-10-24
lane: glue-overlays-shadow5
dll: FreeCity4-glue-overlays-shadow5; sha256 prefix cbe1bf94
pacing: turbo
verdict: clean
switches: overlays_shadow=1, observe=1, popup_dialogs=0; Konradshohe; end_date=2032-01-01; source bd552f9e9
result: Clean at 2032-01-13, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0; 34.781 simulated-run real seconds, 50 s harness wall. Exact log: overlays contract shadow: 72/73 lines exact mismatches=100 fields=7300 scope=sentinels records=blocked(OV12). The PostCityInit load sample found no managers (0/100); all 72 monthly samples were 100/100. Diagnostic evidence retained; load sampling deferred until the first framework tick with all four managers in the next source revision. This does not verify live overlay records/cells/textures. Request and evidence: build-runs/game-queue/done/overlays-shadow5-r1-2032a.req and build-runs/game-results/overlays-shadow5-r1-2032a.{verdict,log}.
