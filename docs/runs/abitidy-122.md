tag: abitidy-122
run: 122
date: 2026-10-16
lane: abitidy
dll: FreeCity4-abitidy
pacing: turbo
verdict: PASS
switches: main `b435187` (abitidy: the ABI conformance test's trailing deleting destructor rule (one extra vtable slot that is the implementing class's deleting destructor popping 4 passes without a reviewed entry; 9-case self-test), `cFC4ISC4TutorialSystem` declares tutorial slots 18-20, the held interfaces.tsv rows applied (occupant manager 49, prop manager 40, lot configuration manager 28, tutorial 18-20); test and docs only, the plugin change is a header declaration nothing calls; ABI 57 unchanged; DLL `FreeCity4-abitidy`, sha256 `69029d1e`), `tools/regress.ps1 -Tag abitidy -Queue` (turbo to 2032-01-01, lists as run 118; baseline picked: rl3)
result: **PASS, 0 problems, 264 s.** Both runs clean, `ref_warnings=0`. Replace 10.08 days/s, shadows 9.23 days/s (machine loaded): zones 292/292, tract 145/145, zonedev 365/365, history 79/79, clock 73/73, demand 2242/2242, parks 72/72, budget 72/72, power 368/368, plumbing 144/144, development 913/913, protection 288/288, flora 72/72, residential 1139/1139, commercial 12/12, civic 72/72, flammability 54/54, every other shadow exact except **land value 71/72** (the usual 2026-02-07 pass 2 early-settling line, 3 of 4096 intrinsic cells, max
summary: regress abitidy, PASS, lists as run 118
