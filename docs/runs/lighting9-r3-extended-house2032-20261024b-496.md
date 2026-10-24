tag: lighting9-r3-extended-house2032-20261024b-496
run: 496
date: 2026-10-24
lane: glue-lighting-shadow9
dll: FreeCity4-glue-lighting-shadow9 (source 0023e8fee; SHA256 42A0D20EE5A9FEE4734588430BDF2779C320B1451D4DEC661E90191863ABAB93)
pacing: turbo
verdict: clean
switches: unchanged run495 house profile plus lighting_shadow=1; Konradshohe; disable=budget,ordinance; end_date=2032-01-01; seconds=1500; timeout=1800
result: ACCEPTED: clean shutdown through2032-01-01 (julian2463233), real950.875s, wall980s, 2.27days/s; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. Original73/73 captures,1022/1022 words exact; additional queries73/73 captures,4818cases,44530/44530 words exact; total45552/45552 words exact (624 per capture), from2026-01-27 through2032-01-01, resyncs=0. Slot16(nonnull) covers all64 property-presence masks with scalar u32/f32 and pair/RGB array inputs; slot17 covers count0 at flags0 and7, false return and all64 output bytes untouched. Original slots14 and16(null) comparison runs after query scenarios and remains exact. Independent resource presence=2F, texture=69D3A23B. Whole-owner comparison remains unavailable under LT14 section1.6: slots0-13,15,18-29, nonempty slot17 and cImageARGB8 are not compared. Preserved evidence build-runs/game-results/lighting9-r3-extended-house2032-20261024b.log and .verdict; notification wait39621 collected to terminal, no qwait, duplicate request or discarded completed verdict. No normal-speed retry required: corrected turbo comparison exact; prior difference was the fixture's property shape.
