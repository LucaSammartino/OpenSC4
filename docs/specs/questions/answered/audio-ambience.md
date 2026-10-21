# audio-ambience: answered questions

- **AA2 - section 3.4:** what does `_UpdateLotOccupantsForConditionChange` (`0x004E9DA0`) start or stop per lot
  condition, for properties `0xCA19D7CA`, `0x0A4C13CB`, `0x4A4C132E`?
  **ANSWERED 2026-10-21 (re-audioambience4):** DIFFERS (no engine rule was stated) — by new condition (data2; insert/init pass -1 = current): 0/5/6 stop 0x0A4C13CB, start 0xCA19D7CA; 2/3 stop 0xCA19D7CA, start 0x0A4C13CB; 1/4 stop both; new 0 with old (data1) 1 also starts 0x4A4C132E; all loops, building or prop occupants of the lot. audio-ambience.md 3.4.

- **AA3 - section 3.6:** confirm the stack slot copied into the alarm entry `+0x10` at `0x004E91B0` is the time
  of day `t` (the listing reuses `[ESP+0x18]` for `a[i]`).
  **ANSWERED 2026-10-21 (re-audioambience4):** MATCHES — yes: `t` is stored at 0x004E8F12 (depth 0x4C) and read back at 0x004E91AC as [ESP+0x1C] after PUSH EBX; the loop's [ESP+0x18] is a different slot (a[i]). audio-ambience.md 3.6.
- **AA1 - section 8:** the ambience layer's own behaviour (scores, loops, filters, per-layer tuning,
  `0x004E5110`-`0x004E8230`) is unspecified; only the interface the manager calls is. Needed before
  `cSC4AudioAmbienceLayer` can count as specified. (re-audioambience4: now §8.1–8.5 and §8.7 cover Init,
  tuning, filters, `CanHandle`/`HandleCell`/`HandleOccupant`, group layers, `Update` and the loops; still open:
  `_UpdateAmbienceScores` `0x004E5920` (§8.6), the ease time step `0x004F8650`, director `+0x54`/`+0x11C`.)
  **ANSWERED 2026-10-21 (re-audioambience4):** DIFFERS (no engine rule was stated) — §8.6: freshness `+0xDC = (trunc(curve1B0(s))·+0x1E8)>>10` (logical), `+0xE0 = trunc(curve1A0(s))` scaled 50%/75% by `+0x204` 0/1; per score start (MinMaxDistance trunc(d), trunc(16d); Play head-relative when `+0x34=4`), stop (fade) when not (vol≠0, pct≠0, unpaused), divisions every update, volume/percent on change. §8.7: ease values are per-ms rates; Step moves min(remaining, t·rate) along the line, scalar integer, flag `+0x38` set. §8.5 mode 6: director `+0x54` is never written, the read faults.
