# audio-ambience: answered questions

- **AA2 - section 3.4:** what does `_UpdateLotOccupantsForConditionChange` (`0x004E9DA0`) start or stop per lot
  condition, for properties `0xCA19D7CA`, `0x0A4C13CB`, `0x4A4C132E`?
  **ANSWERED 2026-10-21 (re-audioambience4):** DIFFERS (no engine rule was stated) — by new condition (data2; insert/init pass -1 = current): 0/5/6 stop 0x0A4C13CB, start 0xCA19D7CA; 2/3 stop 0xCA19D7CA, start 0x0A4C13CB; 1/4 stop both; new 0 with old (data1) 1 also starts 0x4A4C132E; all loops, building or prop occupants of the lot. audio-ambience.md 3.4.

- **AA3 - section 3.6:** confirm the stack slot copied into the alarm entry `+0x10` at `0x004E91B0` is the time
  of day `t` (the listing reuses `[ESP+0x18]` for `a[i]`).
  **ANSWERED 2026-10-21 (re-audioambience4):** MATCHES — yes: `t` is stored at 0x004E8F12 (depth 0x4C) and read back at 0x004E91AC as [ESP+0x1C] after PUSH EBX; the loop's [ESP+0x18] is a different slot (a[i]). audio-ambience.md 3.6.
