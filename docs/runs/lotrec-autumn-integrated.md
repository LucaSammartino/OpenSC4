tag: lotrec-autumn-integrated
run: 388
date: 2026-10-24
lane: glue-simgrid-lotrec5
dll: f4b8f269e / FreeCity4.dll sha256 prefix 54d7c5ee
pacing: turbo
verdict: clean
switches: simgrid_shadow=1, lotrec_shadow=1, popup_dialogs=0, close_dialogs=1; Konradshohe to 2032-01-01
result: clean to 2032-01-01, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; all harness problem counts zero; real_seconds=40.484, wall=56 s. simgrid 73/73 and lotrec 73/73 observations exact, no nonzero mismatches. Final simgrid roles=86 expected=86 developers=12 exact=86/86 mismatches=0 differences=0 unavailable=0 cells=311552 bytes=423538. Final lotrec expected=671 visited=671 captures=671 exact=671/671 mismatches=0 differences=0 errors=0 empty=0 bytes=238616. Existing completed verdict retained; no resubmission.
summary: Round 2 SG3/LR12 integration, exact record observations through 2032
