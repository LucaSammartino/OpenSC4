# mysims: answered questions (archive)
<!-- Archive for [mysims.md](../../mysims.md). Lanes do not read this file. Open questions are in ../mysims.md. -->

## From the mysims core audit (cores26, 2026-10-20); answered by re-answers35

- **MS1 (§11.2.1, §6.2 rows 25 and 30):** the unnamed s32 pair (record row 25, version ≥ 17) and the unnamed bool
  (row 30, version ≥ 14) have no observed meaning. [inferred: the pair is the idle offset; the bool is UI state]
  **ANSWERED 2026-10-21 (re-answers35):** DIFFERS — row 25 (MySim+0x204/+0x208) is dead data: zeroed in the constructor (0x00429A9E), written/read by Write/Read, and touched by no other cSC4MySim method (asm scan of all 69); row 30 (MySim+0x245) is the "pylon locked" flag set by DispatchToolLockPylon (0x0042728E) and gating is_pylon_location, DispatchToolEnd, the pylon timer and UpdateTripVariables. Neither is idle offset or UI state. Spec §2.2, §6.2 rows 25/30.
