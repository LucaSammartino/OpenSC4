tag: glue20-sh-3
run: 144
date: 2026-10-16
lane: glue20
dll: FreeCity4-glue20
pacing: normal
verdict: clean
switches: lane `cc492e4d` (§6.5's leader list walked with the next node remembered, the riot half of DS40) + the earlier glue20 commits; `disasters_shadow = 1`, `script = build-runs/glue20-script-3.txt` (the same script as `glue20-sh-1` and glue13's `dis-final-11`), Konradshohe to 2026-05-01, 600 s timeout
result: **clean, `ref_warnings=0`, 600.8 s.** `disasters shadow: city shutdown: 612 lines, 608 with mismatches=0 (fire 212/215, riot 396/396, quiet 0/1); lines whose every difference is a numbered question: fire 3, riot 0, quiet 1; original steps 205, ours 206; differences by kind: fire ongoing orig [Q40] 1, fire phase orig [Q40] 3, quiet 49D15B73 data1 sum orig [Q40] 1, quiet [Q40] 1, quiet posts [Q40] 6`. **Riot 396/396 exact** (glue13: 2/354; `glue20-sh-1`: 305/324; `glue20-sh-2`: 363/395): a leader step 4 spawns is now processed in the same pass when its spawner was not the last node, so the leader counts track the original's at every spawn boundary. **Fire 212/215 exact** (glue13: 211/263); the three non-exact lines and the quiet line are all `[Q40]` and all come from the one ignition the script provokes at (0, 36) (`disaster fire 0 36`), which re-answers23 leaves **partly open**: `type=278128A0 flammability=75 rect=0,36..0,36 coverage=0 burning=0 paused=0 emergency=0 -> orig 0 ours 1 ... exits=none-of-1,2,3,5`. The cost-record compare (DS39) is exact: `cost record buildings 3/3 total 540/540`.
summary: disasters shadow after the riot walk fix; riot 396/396 exact, fire 212/215
