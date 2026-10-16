tag: glue20-fire-1
run: 145
date: 2026-10-16
lane: glue20
dll: FreeCity4-glue20
pacing: normal
verdict: clean
switches: lane `cc492e4d` plus the kept-reference prune (a reference the shadow keeps is dropped as soon as its occupant leaves the occupant manager, instead of at city shutdown); `disasters_shadow = 1`, `script = build-runs/glue20-script-4.txt` (`disaster survey`, three `disaster fire auto`, `disaster riot auto` - no deliberately refused edge ignition), Konradshohe to 2026-05-01, 600 s timeout
result: **clean, `ref_warnings=0`, 600.7 s.** `disasters shadow: city shutdown: 611 lines, 570 with mismatches=0 (fire 303/343, riot 267/267, quiet 0/1); lines whose every difference is a numbered question: fire 39, riot 0, quiet 1; original steps 334, ours 334; differences by kind: fire fires orig [Q40] 1, fire locations orig [Q40] 38, fire lot 1, fire occupant [Q40] 1, quiet 49D15B73 data1 sum orig [Q40] 1, quiet [Q40] 1, quiet posts [Q40] 2`. **Riot 267/267 exact.** The fire's non-exact lines all come from one ignition the original refused (`disaster fire auto`'s survey candidate): `Ignite (in the original's fire step) ... type=278128A0 flammability=40 rect=56,86..56,87 coverage=1 burning=0 paused=1 emergency=1 -> orig 0 ours 1 ... exits=none-of-1,2,3,5` - a **spread** target, exactly the case re-answers23's DS38 leaves open (its likeliest exit is step 5, a target that already burns in the original's own pass; the logged `burning` is read at mirror time, after that pass). The one untagged `lot 152BC7B8 condition orig 6 ours 5` is a one-step lag at that fire's stage-0 to stage-1 boundary: a fire a spread lights is mirrored after the mirror's own step of the tick, so its stage - and the lot condition that follows it - can be one step behind (tagged `[Q42]` afterwards). `cost record buildings 1/1 total 140/140` exact.
summary: disasters shadow, riot 267/267 exact, fire 303/343 (DS38 + a deferred-spread lot lag)
