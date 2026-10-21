| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-streams4: clean-room Rust core for the RZ stream owners (output-streams spec §1-§5) | Claude lane / .claude/worktrees/core-streams4 | ready: engine/core/src/rz_streams, 22 tests, gate green; no plugin/CAPI, no game run | glue lane to wire cRZNullOStream/cRZNullIStream/cRZOFStream; answers to OS3-OS6 |

## Report

### What changed
- New module `engine/core/src/rz_streams/` (registered in `engine/core/src/lib.rs`), pure Rust, no plugin/CAPI:
  - `com.rs` `StreamCore`: count, littleEndian flag, counted user data (`CountedRef`); `QueryInterface` (1 + own IID),
    `Release` quirk (deletes at count <= 1 without decrementing, `ReleaseOutcome`), `SetUserData`/`GetUserData`,
    `Init`, `Shutdown`, destructor releases user data (§1.1, §1.2, §1.6).
  - `ostream.rs` `GzOStream` trait (slots 3-18 + 16), `RzOStream<S: ByteSink>` (cRZOStream §1.3-§1.5: byte order,
    floats never reversed, LEB128 string length through virtual SetUint8, string result quirk),
    `RzNullOStream` (§3, flag false).
  - `text.rs` `RzTextOStream<T: TextSink>` (§2): decimal text, `format_g` (MSVC `%g`, 3-digit exponent,
    INF/QNAN/IND/SNAN, locale decimal point parameter), hex dump with sign-extension and stop-on-failure, GetError 1.
  - `file.rs` `RzOFStream<F: OfFile>` (§4) over a typed `OfFileOpener`/`OfFile` provider (+ `StdFileOpener`):
    ofstream state bits, sticky errors, partial transfer = badbit, binary/text choice per slot, raw binary strings,
    autoFlush after strings/text values only, failed open = failbit.
  - `istream.rs` `GzIStream`, `RzIStream<S: ByteSource>` (§5: reversal on success only, output keeps what GetVoid
    left on failure, GetRZCharStr terminator at buf[k], Skip of the remainder, GetGZStr in 255-byte chunks, length
    decoder) and `RzNullIStream` (error byte).
- Questions OS3-OS6 appended to docs/specs/questions/output-streams.md.

### Measured
- 22 rz_streams tests (all §9 vectors 1-14, plus format_g edges, file partial transfer/stickiness, autoFlush and
  text mode, short reads on integers/strings/chunks, long varint, null istream).
- Gate (`qbuild all`): cargo test 4002 passed / 0 failed; clippy -D warnings 0; CMake 35/35; ctest 65/65.

### Owners: complete vs remainder
- Complete in core: cRZOStream, cRZNullOStream, cRZTextOStream, cRZIStream, cRZNullIStream, cRZOFStream (file sink).
- Not done (out of scope for this lane): plugin/CAPI glue, shadow/replace, conformance vectors in
  engine/conformance/vectors/, the DBPF record streams of §6 (other owners).
- Not modelled: the ofstream allocation failure at +0x10 (no Rust counterpart).

### Questions
- OS3 `%g` tie rounding; OS4 Flush after failed open; OS5 length decoder failed byte, GetGZStr failed chunk;
  OS6 Skip(0) on exact fit, cRZNullIStream flag.

### Next
- Glue lane: expose via CAPI and back the framework's default null streams / GZ log stream; then a game shadow.
