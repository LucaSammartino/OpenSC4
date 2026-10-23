# Answered GZ graphics questions

- **GZG6 — context destruction versus replacement.**
  **ANSWERED 2026-10-23 (re-gzgraphics-gzg5):** DIFFERS — §2.1 now specifies
  explicit engine Shutdown, temporary-vector cleanup, engine Release, then
  independent target Release. Instruction evidence: `0x0082CA40`–`0x0082CADD`.
  Direct final Release of a buffer engine still has the separate no-unlock
  quirk; context destruction explicitly shuts that engine down first.
