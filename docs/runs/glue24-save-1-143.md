tag: glue24-save-1-143
run: 143
date: 2026-10-16
lane: glue24
dll: FreeCity4-glue24
pacing: turbo
verdict: clean
switches: `netmgr_shadow = 1`, `script = build-runs\scripts\glue24-save.txt` (mark, digest, `save`, mark, digest), turbo to 2027-01-01 on Konradshohe, queue request `glue24-save-1`
result: **clean, ref_warnings=0, 31 s wall, 19.56 days/s, reached 2027-01-04.** The **component 53 and 11 save round trip**: `netmgr shadow (load) 2026-01-27: mismatches=0 ... managed=0/0 record53=equal (6 bytes); lots=525/525 potholes=0/0 record11=equal` and `netmgr shadow (save) 2026-03-10: mismatches=0 ... record53=equal (6 bytes); lots=529/529 potholes=0/0 record11=DIFFERS`. Both records re-read and re-written by the shadow core against the original's own bytes at load and again immediately before `SaveCity`; the script's `save` returned `ok=1 (SaveCity(full) in 11015 ms)`. Record 53 (the managed list) is byte-equal at both points. Record 11 (the network lots) matches entry by entry at both points (`lots=529/529`, `potholes=0/0`); its `DIFFERS` is one **clock-seeded configuration draw** (`draws=1`, the original's generator is seeded from the clock, network-manager.md 8.5/17: counted, not a mismatch) plus the **NM10** open question (`NM10 differences: style at 49,77 and 54,88: variation 5/6`, `resynced: lot`).
summary: component 53 / 11 save round trip, record53 equal at load and save
