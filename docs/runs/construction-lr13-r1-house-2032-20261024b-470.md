tag: construction-lr13-r1-house-2032-20261024b-470
run: 470
date: 2026-10-24
lane: glue-construction-lr13
dll: c44e2ba59 production / cbf50be94 fixture (DLL sha256 prefix f7fe29c4)
pacing: turbo
verdict: corrupt
switches: house shadows profile from regress-train187-shadows.ini; automata_owners_shadow=1; end_date=2032-01-01; seconds=900
result: automataowners 12/13 including load (dated 11/12); construction 107/108 exact. On 2027-01-01 field 13 captured bare class 0xA9BC9AB6, rejected effect IID 0x2977AA49, strict replay 254/250 bytes, followed by last-reference warning on capture-pin cleanup. Foreign exception on 2027-01-09, foreign_exceptions=1/ref_warnings=1, 114.703 real seconds. LR14 filed; no interface coercion or length normalization. Log and verdict: build-runs/game-results/construction-lr13-r1-house-2032-20261024b.*.
summary: LR13 rejected-reference mismatch reproduced; LR14 open, verification incomplete
