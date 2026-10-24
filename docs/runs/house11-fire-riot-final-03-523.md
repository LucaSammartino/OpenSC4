tag: house11-fire-riot-final-03-523
run: 523
date: 2026-10-24
lane: glue-disasters-house11
dll: FreeCity4-glue-disasters-house11 source81b5d3024 sha256-prefix0311f5eb
pacing: turbo
verdict: clean
switches: train193 house shadows plus disasters_shadow=1 disasters_scripted=1; disable=budget,ordinance; end_date2032-01-01 seconds1740 timeout1800; one unchanged-source retry after stalled run493; both disaster switches default-off
result: READY measured partial closure, house profile NOT qualified. Exact countable line: disasters 73/73. Reached2032-01-02 and PostSystemServiceShutdown in1019.469s (queue wall1042s), foreign_exceptions0 watchdog_stalls0 rtc0 overruns0 ref_warnings1, no panics or structured exceptions. Episode comparisons1071/1071 exact: fire65/65 riot941/941 quiet65/65, original/ours steps55/55; resyncs0 unfollowable0 failures0. Schedule fire(9,70), riot(40,33), nine leaders removed after200 samples, completed58fire/202riot. One last-reference warning PruneGone:275 class0x2977AA47 object187DDAE8. Propmgr219/220 versus train193220/220: only nonzero query2031-05-01 lot(23,53)4/3 listsDiffering1 resultsDiffering0; landvalue72/72 versus71/72; all other51 non-disaster groups exact. Candidate52/53 groups exact, baseline51/52. Final-source qbuild all PASS Rust7119/0 clippy0 native47/47 CTest81/81; no source changed after gate. Default-off measured work ready to land under final-round partial-closure instruction; no profile inclusion or ref0 qualification claim. Existing DS28/DS47 open, no questions added this round.
summary: house11 final disasters73/73 to2032, clean1019.469s; partial closure with ref1 and propmgr219/220 outstanding

All dated mismatch-bearing shadow groups (all numeric mismatch fields must be zero, including `mismatched`). Ordered payloads and nonzero lines: `build-runs/house11-final-03-comparison.txt`; terminal log/verdict: `build-runs/game-results/house11-fire-riot-final-03.*`. Baseline: authorized main `build-runs/regress-train193-shadows.log`.

```text
advisors 73/73 (train193 73/73)
aura 72/72 (train193 71/71)
automata 72/72 (train193 72/72)
automataowners 72/72 (train193 72/72)
budget 72/72 (train193 72/72)
citydetail 1/1 (train193 1/1)
civic 72/72 (train193 72/72)
clock 73/73 (train193 73/73)
commercial 16/16 (train193 22/22)
crime 71/71 (train193 71/71)
dbpf 73/73 (train193 73/73)
demand 2241/2241 (train193 2241/2241)
development 925/925 (train193 913/913)
disasters 73/73 (train193 0/0)
feature 75/75 (train193 75/75)
flammability 54/54 (train193 54/54)
flora 72/72 (train193 72/72)
history 79/79 (train193 79/79)
landfill 72/72 (train193 72/72)
landvalue 72/72 (train193 71/72)
lotcfg 293/293 (train193 293/293)
lotconf 73/73 (train193 73/73)
lotdev 73/73 (train193 73/73)
lotrec 73/73 (train193 73/73)
lua 73/73 (train193 73/73)
mysims 73/73 (train193 73/73)
neighbours 72/72 (train193 72/72)
netmgr 73/73 (train193 73/73)
netocc 72/72 (train193 72/72)
netrules 72/72 (train193 72/72)
occmgr 72/72 (train193 72/72)
ordinance 72/72 (train193 72/72)
overlayrecords 292/292 (train193 292/292)
overlayreplay 292/292 (train193 292/292)
overlays 73/73 (train193 73/73)
parks 72/72 (train193 72/72)
plumbing 144/144 (train193 144/144)
pollution 72/72 (train193 72/72)
power 382/382 (train193 390/390)
propmgr 219/220 (train193 220/220)
protection 288/288 (train193 288/288)
residential 1137/1137 (train193 1136/1136)
scres 149/149 (train193 149/149)
simgrid 73/73 (train193 73/73)
strings 72/72 (train193 72/72)
terrain 592/592 (train193 593/593)
tract 145/145 (train193 145/145)
traffic 72/72 (train193 72/72)
tutorial 146/146 (train193 146/146)
variant 1/1 (train193 1/1)
weather 73/73 (train193 73/73)
zonedev 365/365 (train193 365/365)
zones 292/292 (train193 292/292)
```
