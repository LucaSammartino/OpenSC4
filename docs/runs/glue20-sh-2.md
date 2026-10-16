tag: glue20-sh-2
run: 143
date: 2026-10-16
lane: glue20
dll: FreeCity4-glue20
pacing: normal
verdict: clean
switches: lane `0a792167` (the shadow closes the resync episode inside the mirrored call, not at the next framework tick) + the earlier glue20 commits; `disasters_shadow = 1`, `script = build-runs/glue20-script-2.txt` (the same script as `glue20-sh-1`), Konradshohe to 2026-05-01, 600 s timeout
result: **clean, `ref_warnings=1`, 600.7 s.** `disasters shadow: city shutdown: 615 lines, 578 with mismatches=0 (fire 215/219, riot 363/395, quiet 0/1); lines whose every difference is a numbered question: fire 4, riot 32, quiet 1; original steps 205, ours 207; differences by kind: fire ongoing orig [Q40] 2, fire phase orig [Q40] 4, quiet 49D15B73 data1 sum orig [Q40] 1, quiet [Q40] 1, quiet posts [Q40] 6, riot leaders orig [Q39] 32, riot locations orig [Q39] 31`. **Every `[Q37]` is gone** - an ignition the original refuses no longer leaves the mirror in a phantom episode across the next mirrored call, so a fire lit after one starts its own episode and §2.7 resolves its location in the same call. The one `ref_warnings=1` was `WARNING: released last reference to game object ... at fc4::DropAll:250 (the game still expects to own it)`: the shadow held a fire target after the game removed it and released it at city shutdown. Fixed by pruning a kept reference as soon as its occupant leaves the occupant manager (verified `ref_warnings=0` in `glue20-sh-3`).
summary: disasters shadow after the resync fix, clean (one ref warning)
