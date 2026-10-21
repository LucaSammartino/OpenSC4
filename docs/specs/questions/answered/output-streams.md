# output-streams: answered questions (archive)
<!-- Archive for [output-streams.md](../../output-streams.md). Lanes do not read this file. Open questions are in ../output-streams.md. -->

<!-- from: core-streams4 (2026-10-21) -->
- **OS3 - §2.2:** `%g` rounding of exact decimal ties at the 6th significant digit (e.g. `SetFloat64(1234565)`,
  `SetFloat64(0.1234565)` as stored): does the MSVC 7 runtime round half-up on its own digit string (and from how
  many digits), or half-even? We take 17 correctly rounded digits, then round half-up to 6. A tie vector would settle it.
  **ANSWERED 2026-10-21 (re-stream-core-answers4):** MATCHES — double rounding, half-up both times: $I10_OUTPUT 0x00A04942 makes 18 digits and rounds half-up to 17 (fltout2 0x00A0277B passes 17), __fptostr 0x00A0264A rounds half-up on the 7th digit; ties go away from zero (1234565 -> 1.23457e+006, 0.1234565 -> 0.123457, 12345.25 -> 12345.3); residual scaling-error case filed as OS7; output-streams.md §2.2.

<!-- from: core-streams4 (2026-10-21) -->
- **OS4 - §4.2:** `cRZOFStream::Flush` when the open failed (failbit set, no file open): does `ofstream.flush()`
  change the state (sync of an unopened file buffer failing and adding badbit), or nothing? We change nothing;
  `GetError()` is true either way.
  **ANSWERED 2026-10-21 (re-stream-core-answers4):** MATCHES — Flush 0x0092BCF7 -> 0x0092BA02 calls the filebuf sync 0x0092BF91, which returns 0 (no state change) unless the buffer is in output mode, set only by 0x0092C327 on an open file; a null ofstream pointer makes Flush fault; output-streams.md §4.2.

<!-- from: core-streams4 (2026-10-21) -->
- **OS5 - §5:** (a) in the length decoder, when a `GetUint8` fails and leaves its byte unwritten, which value is
  tested for bit 7: a fresh byte (0, ending the loop), the previous byte (continuing), or garbage? We use 0.
  (b) `GetGZStr`: when the `GetVoid` of a chunk fails, is that chunk (or its partial bytes) still appended to `out`
  before the loop stops? We do not append it.
  **ANSWERED 2026-10-21 (re-stream-core-answers4):** DIFFERS — (a) the decoder 0x0092B85F ignores read results and tests the unwritten stack slot, pre-filled with the bytes of the stream object's own address (PUSH ECX x2): unspecified natively, never the previous byte; 0 is a [choice]. (b) GetGZStr 0x0092B92D appends the failing chunk (k bytes, stale buffer bytes: previous chunk's, or uninitialised stack on the first) before testing the error; output-streams.md §5.

<!-- from: core-streams4 (2026-10-21) -->
- **OS6 - §5:** (a) `GetRZCharStr` when `length <= max`: is `Skip(0)` still called (it is on a source whose `Skip`
  records calls or fails on 0), or only when `length > max`? We call `Skip(length - k)` whenever no error.
  (b) which byte-order flag does the `cRZNullIStream` constructor pass to `cRZIStream(bool)`? We pass false, as
  `cRZNullOStream` does (unobservable: every read fails).
  **ANSWERED 2026-10-21 (re-stream-core-answers4):** (a) DIFFERS — Skip is called only when length - k is not 0 (0x0092B906-0x0092B908); Skip(0) never. (b) MATCHES — the cRZNullIStream ctor 0x0092BC80 stores the fields inline, flag +0x08 false; output-streams.md §5.

