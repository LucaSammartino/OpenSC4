# widget-toolkit: open questions
<!-- Questions about [widget-toolkit.md](../widget-toolkit.md). Append new entries at the END; when one is answered, move it to answered/widget-toolkit.md. Rules: README.md. -->

<!-- from: re-widgets4 (2026-10-21) -->
- **WT1 - section 1:** the cGZWin own vtable 0x00ADC8D8 (written by both constructors and the destructor) has no rows in docs/abi/vtables.tsv; how many slots does it have and which are pure stubs? [assume the cIGZWin 148-slot layout plus slots 148-151 as on cGZWinGen]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT2 - section 2:** what are cGZWin +0x0C, +0x24..+0x30, +0x34..+0x40, +0xA0..+0xA4 and +0xB8..+0xC4 (the constructor zeroes some, leaves +0x34..+0x40 and +0xA0..+0xA4 untouched)? Candidates: area-to-draw-to and draw-buffer origin. [unknown; implementers should zero them]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT3 - section 3:** default flag bits 0x100 and 0x8000 are set by the constructor but not tested by cGZWin itself; which subclass or manager code reads them? [unknown; keep them set]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT4 - section 3.1 (and ui-windows.md section 2.2):** cGZWin::SetFlag 0x0099DB6B returns the whole flag word in EAX (0x0099DCD9), not 0/1 as ui-windows.md section 2.2 states; should ui-windows.md be corrected to point here? [yes]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT5 - section 4.2:** cGZWin::ChildDeleteAll 0x0099DD6F loops on the current front child until the list is empty; does cIGZWinMgr::DestroyWindow (slot 23) always detach the child from its parent synchronously, also when the manager is busy and defers the destroy? If not, the loop would not terminate. [assume it detaches synchronously]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT6 - section 8.1:** what is the initial value of the window manager's instance-ID counter (manager +0x194) assigned by registration (manager slot 20, 0x009DC586)? [unknown; first ID may be 0 or 1]
