tag: abitidy2-140
run: 140
date: 2026-10-16
lane: abitidy2
dll: FreeCity4-abitidy2
pacing: turbo
verdict: PASS
switches: main `6da51f0` (abitidy2: the held interfaces.tsv rows applied (cISC4LotConfiguration slot 33 from re-specs15a; cIGZCommandParameterSet slots 7, 8 and 10 from re-answers12 CB16), `cFC4IGZCommandParameterSet` declared in `FC4Interfaces.h` and checked by the ABI conformance test (`gen_abi_entries.py`, `abi_entries.inc` 178 interfaces), five `kReviewedSlotCounts` entries dropped for the trailing destructor rule; test and docs only, no runtime code, ABI stays 68; DLL `FreeCity4-abitidy2`, sha256 `3abdc8d8`), `tools/regress.ps1 -Tag abitidy2 -Queue` (turbo to 2032-01-01, lists as run 138; baseline picked: rl5)
result: **PASS, 0 problems, 1234 s** (a loaded machine: shadows 1.84 days/s, 1170 s; replace 10.26 days/s). Both runs clean, `ref_warnings=0`, rtc=0. Shadows: zones 292/292, tract 145/145, lotdev 73/73, zonedev 365/365, history 79/79, clock 73/73, terrain 93/93 (totals 7/7 exact; 0 unfollowed, 0 late, 0 pending), demand 2237/2237, protection 288/288, parks 72/72, budget 72/72, power 393/393, plumbing 144/144, land value 72/72, flora 72/72, pollution 71/71, civic 71/71, landfill 71/71, neighbours 71/71, residential 1138/1138, crime 71/71, aura 71/71, commercial 28/28, flammability 54/54, **traffic 53/91** (known difference, hypotheses 7), **development 911/913** (growth 0x1020 on 2028-03-02 and 2028-05-02: population at 5 and 1 tracts differing, max \
summary: regress abitidy2, PASS, lists as run 138
