# Answered GZ graphics questions

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
