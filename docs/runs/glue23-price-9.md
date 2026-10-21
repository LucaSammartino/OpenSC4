tag: glue23-price-9
run: 161
date: 2026-10-16
lane: glue23
dll: FreeCity4
pacing: turbo
verdict: clean
switches: demolition_price_shadow = 1, script = build-runs/glue23-demolish.script, turbo to 2032-01-01 on Konradshohe
result: **clean**, ref_warnings=0, rtc=0, overruns=0, 62.83 days/s. **The pricing differential (glue17's missing proof): `demolition price shadow: city shutdown: 73 lines, 73 with mismatches=0; compared 501760 prices, 0 mismatched; differences: none`.** Every DemolishOccupant price of every occupant the occupant manager lists over the city, ours against the original's on the same live city, matched exactly (cost and the bool the call returns) at PostCityInit and every SimNewMonth. Our own whole-city DemolishRegion dry run also ran every month (ok=1).
summary: glue23 demolition pricing differential, clean, 501760 prices compared 0 mismatched
