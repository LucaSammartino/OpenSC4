# mysims: answered questions (archive)
<!-- Archive for [mysims.md](../../mysims.md). Lanes do not read this file. Open questions are in ../mysims.md. -->

## From the mysims core audit (cores26, 2026-10-20); answered by re-answers35

- **MS1 (§11.2.1, §6.2 rows 25 and 30):** the unnamed s32 pair (record row 25, version ≥ 17) and the unnamed bool
  (row 30, version ≥ 14) have no observed meaning. [inferred: the pair is the idle offset; the bool is UI state]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS — row 25 (MySim+0x204/+0x208) is dead data: zeroed in the constructor (0x00429A9E), written/read by Write/Read, and touched by no other cSC4MySim method (asm scan of all 69); row 30 (MySim+0x245) is the "pylon locked" flag set by DispatchToolLockPylon (0x0042728E) and gating is_pylon_location, DispatchToolEnd, the pylon timer and UpdateTripVariables. Neither is idle offset or UI state. Spec §2.2, §6.2 rows 25/30.

- **MS2 (§11.2.2, §4.2):** UI service `0xA417445E` (instance `0x5A4`) slot `+0xA8` — which window or state blocks
  the MySim daily and monthly steps? [inferred: a modal window]
  **ANSWERED 2026-10-21 (re-answers35):** MATCHES — service 0xA417445E is the window manager (kGZWinMgrDefaultSysServiceID, cIGZWinMgr IID 0x5A4); slot 42 (+0xA8) resolves to 0x009DB025, which returns (modal depth at WinMgr+0x17C) != 0, the counter cGZWinMgrBase::DoModalWin increments/decrements around its loop (0x009DBF11/0x009DBF2D). simulate_day (0x0042DCD9) and simulate_month both skip while any modal window runs. Spec §4.2.
