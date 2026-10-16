tag: glue20-sh-1
run: 142
date: 2026-10-16
lane: glue20
dll: FreeCity4-glue20
pacing: normal
verdict: clean
switches: lane `3656e20e` (the DS25-DS40 answers applied to `engine/core/src/disasters/` and the shadow: DS40's `UpdateAllFires` walk with the next node remembered, DS35's location write-back and slot 17 not resolving, DS38's unsigned flammability read, DS34's toast flammability test, DS39's statistics-record compare) and `90e8531` (DS26 and DS27 wired live: the fire occupant `0x69D25C5F` and the riot leader automata `0x4107`); `disasters_shadow = 1`, `script = build-runs/glue20-script-1.txt` (glue13's `dis-script-4` shape: `disaster survey`, `disaster fire 0 36` - an edge building the original refuses - three `disaster fire auto`, `disaster riot auto`), Konradshohe to 2026-05-01, 600 s timeout
result: **clean, `ref_warnings=0`, 600.8 s.** `disasters shadow: city shutdown: 611 lines, 534 with mismatches=0 (fire 229/286, riot 305/324, quiet 0/1); lines whose every difference is a numbered question: fire 57, riot 19, quiet 1; original steps 277, ours 278; differences by kind: fire fires orig [Q40] 1, fire location [Q37] 55, fire locations orig [Q40] 22, fire occupant [Q40] 1, fire ongoing orig [Q40] 2, fire phase orig [Q40] 2, quiet 49D15B73 data1 sum orig [Q40] 1, quiet [Q40] 1, quiet posts [Q40] 2, riot leaders orig [Q39] 19, riot locations orig [Q39] 18`. Against glue13's `dis-final-11` (fire 211/263, riot 2/354): fire 229/286, riot 305/324. The 55 `[Q37]` location lines were traced to the resync (see `glue20-sh-2`); the 19 `[Q39]` riot leader lines to the leader-list walk (`glue20-sh-3`).
summary: disasters shadow before the resync and riot-walk fixes, clean
