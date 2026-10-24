tag: glue-lots-shadow12-house2032-normal-r2-518
run: 518
date: 2026-10-24
lane: glue-lots-shadow12
dll: FreeCity4-glue-lots-shadow12 c1f97ad6
pacing: normal
verdict: clean
switches: lotrec_shadow=1, lottrip_shadow=1; Konradshohe; turbo=0; end_date=2032-01-01
result: glue-lots-shadow12-house2032-normal-r2 146/146 (lotrec 73/73, lottrip 73/73), all mismatches=0; reached 2032-01-01 in 114.890 real seconds. foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. Nonempty every observation: 49,811 exact lot records and 66,343 exact inline trip instances across load/month samples. Final lotrec exact=668/668, differences=0, errors=0; final lottrip owner=cSC4TripData lots=668/668 exact=820/820. Business-unit/base-texture/wall payloads remain excluded pending LR15; record-only evidence, no complete-owner replacement claim.

Exact summary from the lane's bounded log accounting (not a run of forbidden tools/coverage.py):

```text
glue-lots-shadow12-house2032-normal-r2 146/146
lotrec: 73/73 clean; exact instances=49811; empty=0
lottrip: 73/73 clean; exact instances=66343; empty=0
```

Final native log lines, with timestamps removed:

```text
lotrec shadow 2032-01-01 month: julian=2463233 expected=668 visited=668 captures=668 exact=668/668 differences=0 errors=0 empty=0 bytes=233708 mode=record-only scope=master-lots excluded=off-list,units,occupants,unspecified-props; mismatches=0
lottrip shadow 2032-01-01 month: owner=cSC4TripData lots=668/668 exact=820/820 empty=0 mode=record-only scope=master-lot-inline-trips excluded=off-list-trips; mismatches=0
```

Evidence: `build-runs/game-results/glue-lots-shadow12-house2032-normal-r2.log` and `.verdict`.
The first turbo attempt is separately preserved as run 511; its watchdog stall did not reach the target.
