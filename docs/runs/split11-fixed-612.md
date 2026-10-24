tag: split11-fixed-612
run: 612
date: 2026-10-24
lane: tool-regress-split11
dll: main ffb8f38c2 engine, rebuilt by qbuild verify; sha256 0a67d38e6a8c1fbf969f7e483132fa17ead29076f3a0d248d8ddff2037ede82c
pacing: turbo to 2032-01-01, queue, ShadowSplit 3
verdict: PASS, zero harness problems; 51/52 shadow labels exact, flammability 53/54 (one mismatch)
switches: unchanged single house shadows profile, no candidate extras; standard replacements
result: regress-split11-s3-fixed wall 664 s; group completion offsets 85/174/660 s, replacement 173 s. All groups had clean harness verdicts. Target under 420 s was not met. The initial timestamp proxies poorly predict cost in the third group. Native verify build 262/262 and ctest 81/81 passed. Actual coverage.read_regress parsed all 52 labels from the aggregate output without changes; coverage.newest_regress correctly selected main's reachable train196 evidence rather than this unmerged lane. Every mismatch total below is zero except flammability=1. No coverage-map or coverage-shadows edits or new default shadow switches.
summary: Measured real-city split infrastructure; performance and strict combined parity remain open.

Counts from `tools/coverage.py read_regress(build-runs/regress-split11-s3-fixed.out)`:

```text
advisors 73/73
aura 71/71
automata 72/72
automataowners 72/72
budget 72/72
citydetail 1/1
civic 72/72
clock 73/73
commercial 42/42
crime 72/72
dbpf 73/73
demand 2241/2241
development 927/927
feature 75/75
flammability 53/54
flora 72/72
history 79/79
landfill 72/72
landvalue 73/73
lotcfg 293/293
lotconf 73/73
lotdev 73/73
lotrec 73/73
lua 73/73
mysims 73/73
neighbours 72/72
netmgr 73/73
netocc 72/72
netrules 73/73
occmgr 72/72
ordinance 72/72
overlayrecords 292/292
overlayreplay 292/292
overlays 73/73
parks 72/72
plumbing 144/144
pollution 72/72
power 395/395
propmgr 220/220
protection 288/288
residential 1145/1145
scres 149/149
simgrid 73/73
strings 72/72
terrain 594/594
tract 145/145
traffic 72/72
tutorial 146/146
variant 1/1
weather 73/73
zonedev 365/365
zones 292/292
```
