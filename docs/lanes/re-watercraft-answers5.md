| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-answers5 (watercraft owner answers) | Codex / `re-watercraft-answers5` | in progress; WC8 answered, WC3/WC7 partial | Continue full owner trace; do not enqueue until WC1–WC7 gaps are resolved or explicitly reported. |

## Handoff
- Goal: answer WC3-WC8, and WC1/WC2 where evidence permits, without engine edits.
- Done: 9414cde9 initial lane; 94facf3f WC8 answered with command/steering equations and 13 vectors.
- Done: 1b572698 WC3 Init/Read boundaries; WC7 final docking/undocking, signed remainder and backtracking; 75-entry attribution inventory; 16 more vectors.
- Round 2: corrected §2 labels: p[1] is held-input steering gain per ms; p[2] is steering return rate per second (§6.2.2). Corrected movement-helper attribution (crash helpers are separate).
- Round 2: WC6 answered: complete command side effects and shared translator/drive ordering, 10 more vectors. Prior correction commit 7ecb671b.
- Round 2: WC7 answered: compressed route format and exhaustion, station reservation/held-byte ownership, following equations and first-list tow picker; 15 more vectors. WC6 milestone 7f2c873e.
- Round 2: WC7 milestone 1f86fb4d; crash dispatch now specifies double elapsed accumulation, (0xB,0), and null grounding/mushroom failure paths; 4 more vectors. WC5 remains open for motion/collisions.
- Next FIRST: finish WC5 sink/heel helpers (main unnamed region_00560000.c lines2013/2285), verify numeric equations with asm; then WC3 inventory contracts, WC4 full motion, WC1/WC2.
- Main attribution now gives 75 functions/38073 bytes, versus brief 79/38556. Read-only reproducer: `build-runs/watercraft-owner-inventory.py 0` and `40`; imports Attribution only, no engine reads.
- Numeric PE reader: `build-runs/watercraft-evidence.py <hex VA>...`; reads working exe, prints f32/bits; never commit these scripts or RE output.
- WC8: tan/atan, f32 a=0.910000026226, b=1.57079637051; opposite gear crosses zero; neutral translator keeps speed. Steering channels +88/+84 are elapsed floats, ordered responses; owner never clears skier toggle +EB.
- Docking completion ONLY before motion (00572A57–A87); moving path returns true (00572CE5). Final approach dt unclamped. OnLand pushes72 and continues. Earlier question brackets claiming post-motion completion are wrong.
- Direction helper is signed remainder, NOT wrap: base2 produces2,3,1,4,0,5,-1,6. Step -1 does not move cell. Backtrack triggered on first eligible rejection once per outer round; survivor's own direction supplies tried bit.
- Init flag missing in old spec is 0x8000; base Init/effect creation results ignored; returns true. Timers include lifetime limit reset. Read accepts exactly u16=2, then base result.
- Watercraft decomp offsets: command2152, Init1835, AI pick2878, AI motion3161, docking3464, animation3653; 60-line reads only. Helpers in main decomp/_unnamed/region_00560000.c; grep headers first.
- Crash sequence 00570270 absent in class decomp; asm works. Calls sink helper0056A440, heel0056AF90; body pseudocode at unnamed lines2013/2285. No REA call needed so far.
- Following005694B0 returns false without tug; uses follower radius+10, endpoints helpers00567A50(back)/00567AB0(front); translation appears distance excess with no dt scale. Needs full verification before specifying.
- Movement0056C430 calls00569940(curved),00568230(straight),00569E70(height/tilt); spec's reference to crash helper as position helper is suspect and needs correction.
- Questions: WC6–WC8 moved to answered/watercraft.md; WC1–WC5 remain open. Regenerate QUESTIONS.md at each answer milestone.
- Gate: diff --check passed at WC6/WC7 milestones; inventory75/75 unique; 58 spec-side vectors total (29 prior +10 WC6 +15 WC7 +4 crash dispatch); no engine tests (RE-only), ABI proposals/exports. Round 2 near150k; continuation required, no merge queue.
- Evidence source: main checkout `C:/Users/tlori/FreeCity4`; all edits stay in this worktree.
- No game runs, ABI changes, or engine reads.
