tag: lighting9-r3-extended-house2032-20261024a-508
run: 508
date: 2026-10-24
lane: glue-lighting-shadow9
dll: FreeCity4-glue-lighting-shadow9 (source d441508d5; SHA256 EEF6F4E39B5C00610369738173529A459F35A8BCC3596E30F86D054DA5264C60)
pacing: turbo
verdict: clean
switches: exact previous house profile plus lighting_shadow=1; Konradshohe; disable=budget,ordinance; end_date=2032-01-01; seconds=1500; timeout=1800
result: Native execution clean through2032-01-01 (julian2463233), real873.531s, wall889s, 2.47days/s; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0. COMPARISON FAILED: original73/73 captures and1022/1022 words exact; additional queries0/73 exact, 4818cases,9344 mismatches of44530 words (128 per capture). Four intended scalar properties were mistakenly supplied as one-element arrays, while the independent expected record modeled scalar types;32 occurrences per scalar across64 masks account for all128 mismatches. Pair/RGB arrays and both empty-projection cases exact. Corrected in0023e8fee using typed scalar setters and runtime type validation, per lighting.md section1.6's type/count input requirement. This is an input-fixture failure, not evidence of a turbo-only numeric difference. Preserved evidence build-runs/game-results/lighting9-r3-extended-house2032-20261024a.log and .verdict; notification wait70714 collected to terminal, no qwait or resubmission. Whole-owner unavailable.
