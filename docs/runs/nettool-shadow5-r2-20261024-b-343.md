tag: nettool-shadow5-r2-20261024-b-343
run: 343
date: 2026-10-24
lane: glue-nettool-shadow5
tip: 0027a6926
dll: bc85eb39
city: Konradshohe
duration: 133.250 s, through 2032-01-01
pacing: normal
verdict: clean
switches: enable=simulator; disable=nettool,netmgr; nettool_shadow=1; script=engine/tests/nettool-shadow.script
result: nettool-shadow5-r2-20261024-b 8/14; ref_warnings=0; foreign_exceptions=0; 16.25 days/s. All fourteen comparison lines are identical to turbo run a after stripping timestamps/thread prefixes; the six remaining differences are independent of turbo. NT30 covers the four cost probes (110/113,170/192,90/98,55/61); NT31 covers powerline and bridge placement/validity/cost. No whole-owner coverage or profile enablement. Exact line: nettool shadow (type=6 drag=(4,28)-(16,28) scope=drag/status/cost): mismatches=0 cells=26/26 cell_diffs=0 ok=0/0 failure=00000001/00000001 valid=0/0 straight=13/13 total=13/13 cost=0/0 uncovered=pieces,bridges,poles,wires contract=NT27-NT29
