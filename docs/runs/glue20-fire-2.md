tag: glue20-fire-2
run: 147
date: 2026-10-16
lane: glue20
dll: FreeCity4-glue20
pacing: normal
verdict: clean
switches: lane tip with the deferred-spread lot difference tagged `[Q42]`; `disasters_shadow = 1`, `script = build-runs/glue20-script-5.txt` (`disaster survey`, three `disaster fire auto`, `disaster riot auto`), Konradshohe to 2026-05-01, 600 s timeout
result: **clean, `ref_warnings=0`, 600.7 s.** `disasters shadow: city shutdown: 609 lines, 609 with mismatches=0 (fire 256/256, riot 352/352, quiet 1/1); lines whose every difference is a numbered question: fire 0, riot 0, quiet 0; original steps 249, ours 249; differences by kind: none`. **Every line exact** - fire 256/256, riot 352/352, the quiet point 1/1, and no difference of any kind to tag. Against glue13's `dis-final-11` (fire 211/263, riot 2/354): fire and riot both exact. The run that deliberately provokes an ignition the original refuses (`glue20-sh-3`, `disaster fire 0 36`) is fire 212/215 with the residue all `[Q40]`, that one question re-answers23 leaves open (DS38).
summary: disasters shadow exact - fire 256/256, riot 352/352, no differences
