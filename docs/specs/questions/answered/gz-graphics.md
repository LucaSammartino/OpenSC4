# Answered GZ graphics questions

- **GZG4 — section 7, full Windows canvas event contract:** finish the unnamed
  primary slots and subinterface thunk mappings; settle HWND ownership, window
  creation/reconfiguration failure cleanup, event pump ordering and every
  Windows keyboard/mouse/focus/resize/quit translation. Confirm names/units of
  the two words set by message `0xFA0C6390`, IID `0x1A0C615F`, subtype 13.
  Evidence next: `0x0098C690`, `0x0098CDB0`, `0x0098D080`, `0x0098D230`,
  `0x0098D9C0`, `0x0098DC40`–`0x0098DD70`; instructions are needed where
  adjusted-this and message locals are incorrectly typed in the export.

  **ANSWERED 2026-10-23 (re-gzgraphics-gzg5b):** DIFFERS — §§7.1–7.4 specify
  the native lifetime, adjusted interfaces, ordered retained listeners and full
  window-message translation. Primary slots 7/8 suspend/resume rather than
  attach/remove listeners; slot 15 emits repaint event 4 rather than pumping;
  slot 18 filters/reposts keyboard messages rather than bridging a window proc.
  Stored subtype-13 coordinates are signed client x/y pixels. Realization can
  mark itself realized on failure; the keyboard filter can consume a 1025th
  message without saving/reposting/counting it. Both quirks are contractual;
  primary slot 21 deletes the canvas rather than requesting application quit;
  vectors GZG-V62–V78 cover the newly settled behavior.

- **GZG6 — section 2.1 context destruction versus section 3.4 replacement:**
  Does final context destruction call engine Shutdown before releasing it, or
  only release the independently retained engine and target? Section 2.1 says
  destruction releases both; section 3.4 explicitly requires Shutdown only on
  replacement. The core follows that literal distinction: replacement invokes
  Shutdown, destruction releases engine then target without adding a Shutdown.
  This matters because the buffer engine's final Release does not unlock (§4.2).
  Please settle the context-destructor call sequence; no native destructor
  layout or implicit shutdown guarantee is assumed.

  **ANSWERED 2026-10-23 (re-gzgraphics-gzg5):** DIFFERS — §2.1 now specifies
  explicit engine Shutdown, temporary-vector cleanup, engine Release, then
  independent target Release. Instruction evidence: `0x0082CA40`–`0x0082CADD`.
  Direct final Release of a buffer engine still has the separate no-unlock
  quirk; context destruction explicitly shuts that engine down first.
