# mysims: answered questions (archive)
<!-- Archive for [mysims.md](../../mysims.md). Lanes do not read this file. Open questions are in ../mysims.md. -->

## From the mysims core audit (cores26, 2026-10-20); answered by re-answers35

- **MS1 (§11.2.1, §6.2 rows 25 and 30):** the unnamed s32 pair (record row 25, version ≥ 17) and the unnamed bool
  (row 30, version ≥ 14) have no observed meaning. [inferred: the pair is the idle offset; the bool is UI state]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS — row 25 (MySim+0x204/+0x208) is dead data: zeroed in the constructor (0x00429A9E), written/read by Write/Read, and touched by no other cSC4MySim method (asm scan of all 69); row 30 (MySim+0x245) is the "pylon locked" flag set by DispatchToolLockPylon (0x0042728E) and gating is_pylon_location, DispatchToolEnd, the pylon timer and UpdateTripVariables. Neither is idle offset or UI state. Spec §2.2, §6.2 rows 25/30.

- **MS2 (§11.2.2, §4.2):** UI service `0xA417445E` (instance `0x5A4`) slot `+0xA8` — which window or state blocks
  the MySim daily and monthly steps? [inferred: a modal window]
  **ANSWERED 2026-10-21 (re-answers35):** MATCHES — service 0xA417445E is the window manager (kGZWinMgrDefaultSysServiceID, cIGZWinMgr IID 0x5A4); slot 42 (+0xA8) resolves to 0x009DB025, which returns (modal depth at WinMgr+0x17C) != 0, the counter cGZWinMgrBase::DoModalWin increments/decrements around its loop (0x009DBF11/0x009DBF2D). simulate_day (0x0042DCD9) and simulate_month both skip while any modal window runs. Spec §4.2.

- **MS3 (§11.2.3, §2.2):** does MySim `Init` re-resolve the neighbour job city from the saved cell (the app's
  region lookup), and does it set `neighbor_job` true only when a city is found? [inferred: yes]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS (in part) — yes, Init (0x0042CEF3-0x0042CF44, state != 0 only) re-resolves: if the MySim has no job and the saved neighbour cell is not (0,0), it calls cISC4App slot 35 then cISC4Region::GetCity(x, z) and stores a non-null result; there is no separate neighbor_job bool ("has neighbour job" = the city ref is non-null). What Init sets unconditionally in that branch is the commute-check flag, so the next day's CheckCommute validates the job. With a job or cell (0,0) the ref is released (quirk: cell (0,0) is never re-resolved). Spec §2.2.1.

- **MS4 (§11.2.4, §4.9):** the building development simulator's slot 44 list used by `DistanceToClosestSpecial`
  (§7.2): landmarks and rewards only, or every special building? [inferred: landmarks and rewards]
  **ANSWERED 2026-10-21 (re-answers35):** MATCHES — DistanceToClosestSpecial (0x00426AE0) walks cISC4City slot 53 -> BDS slot 44 (0x00520080, the list at BDS+0x4C), which BDS fills from occupant inserted/removed messages only for occupant groups 0x150A (landmark) and 0x150B (reward), previews excluded (building-development.md §3). Distance is float32 sqrt(dx^2+dz^2) truncated, unsigned minimum from 0xFFFFFFFF; entries without cISC4BuildingOccupant skipped. Spec §4.9.
