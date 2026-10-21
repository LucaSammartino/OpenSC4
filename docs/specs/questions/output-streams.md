# output-streams: open questions
<!-- Questions about [output-streams.md](../output-streams.md). Append new entries at the END; when one is answered, move it to answered/output-streams.md. Rules: README.md. -->

<!-- from: re-ostreams (2026-10-19) -->
- **OS1 - §4.3:** what does the GZ log write to its stream (`SC4CreateGZLog` `0x005C6E80`, the stream handed to the
  framework log)? Only strings, or also numbers, which would come out as raw big-endian bytes? And which file name
  does `cSC4App::PreFrameWorkInit` pass?

<!-- from: re-ostreams (2026-10-19) -->
- **OS2 - §2.2, §4.1:** only for a text-mode file stream, which the game never creates: does the STLport file buffer
  translate `\n` to CR LF in text mode, and which decimal point is in effect when such a stream writes a float (the C
  runtime locale after `0x00910014` and `SetCurrentLanguage`)?

<!-- from: core-streams4 (2026-10-21) -->
- **OS3 - §2.2:** `%g` rounding of exact decimal ties at the 6th significant digit (e.g. `SetFloat64(1234565)`,
  `SetFloat64(0.1234565)` as stored): does the MSVC 7 runtime round half-up on its own digit string (and from how
  many digits), or half-even? We take 17 correctly rounded digits, then round half-up to 6. A tie vector would settle it.

<!-- from: core-streams4 (2026-10-21) -->
- **OS4 - §4.2:** `cRZOFStream::Flush` when the open failed (failbit set, no file open): does `ofstream.flush()`
  change the state (sync of an unopened file buffer failing and adding badbit), or nothing? We change nothing;
  `GetError()` is true either way.

<!-- from: core-streams4 (2026-10-21) -->
- **OS5 - §5:** (a) in the length decoder, when a `GetUint8` fails and leaves its byte unwritten, which value is
  tested for bit 7: a fresh byte (0, ending the loop), the previous byte (continuing), or garbage? We use 0.
  (b) `GetGZStr`: when the `GetVoid` of a chunk fails, is that chunk (or its partial bytes) still appended to `out`
  before the loop stops? We do not append it.

<!-- from: core-streams4 (2026-10-21) -->
- **OS6 - §5:** (a) `GetRZCharStr` when `length <= max`: is `Skip(0)` still called (it is on a source whose `Skip`
  records calls or fails on 0), or only when `length > max`? We call `Skip(length - k)` whenever no error.
  (b) which byte-order flag does the `cRZNullIStream` constructor pass to `cRZIStream(bool)`? We pass false, as
  `cRZNullOStream` does (unobservable: every read fails).
