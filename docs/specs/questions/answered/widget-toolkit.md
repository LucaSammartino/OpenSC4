# widget-toolkit: answered questions (archive)

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTC1 - widget-custom.md section 1:** settle constructor/secondary-vtable and nondeleting teardown attribution around 0095B9A1.
  **ANSWERED 2026-10-24 (re-gzwin-spec9):** MATCHES — constructor 0095BAB3 adds only the custom tables after base construction; the four-slot secondary interface at +0xD8 shares primary QI/reference lifetime and returns the complete pointer through AsIGZWin. Teardown 0095B9A1 directly calls base Shutdown then base destruction. See widget-custom.md section 1 and section 3 vectors.
<!-- Archive for [widget-toolkit.md](../../widget-toolkit.md). Lanes do not read this file. Open questions are in ../widget-toolkit.md. -->

<!-- from: re-widgets4 (2026-10-21) -->
- **WT4 - section 3.1 (and ui-windows.md section 2.2):** cGZWin::SetFlag 0x0099DB6B returns the whole flag word in EAX (0x0099DCD9), not 0/1 as ui-windows.md section 2.2 states; should ui-windows.md be corrected to point here? [yes]
  **ANSWERED 2026-10-21 (re-widgets4):** DIFFERS — SetFlag has one exit (every branch jumps to 0x0099DCD9, MOV EAX,[ESI+0xC8]; RET 8), so it returns the whole new flag word; ui-windows.md section 2.2 row 68 corrected to point to widget-toolkit.md section 3.1.
