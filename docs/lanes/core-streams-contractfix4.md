| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-streams-contractfix4 (cRZIStream GetRZCharStr no Skip(0), GetGZStr failed-chunk append from one reused buffer per answered OS5/OS6; stream owner audit; tests 15-17) | `core-streams-contractfix4` / (engine lane) | ready: 50208211, 4c93414a; no ABI/plugin/CAPI change; gate cargo 4103/0, clippy 0, ctest 65/65; no game run (rz_streams has no plugin/CAPI consumer); no new questions (OS7 stays open) | OS7 (%g 12-byte scaling residual) if ever answered; wire rz_streams into a glue owner before any shadow is possible |

## Report

**Changed** (engine/core/src/rz_streams only):
- `istream.rs` `base_get_rz_char_str`: `Skip(length - k)` only when no error **and** `length - k != 0` (OS6); doc
  states `buf[k] := 0` even on a failed `GetVoid`, caller's prior bytes kept in `buf[0..k]` beyond what it wrote.
- `istream.rs` `base_get_gz_str`: one `[u8; 256]` buffer reused across chunks, `buf[k] := 0`, the `k` bytes are
  appended **before** the error test (failed chunk appended), then stop (OS5). Later failed chunk carries the
  previous chunk's bytes (specified); a failed first chunk's unwritten bytes are 0 by *[choice]*, documented as
  *[compat-limit]*, not a native fact.
- `istream.rs` `decode_string_length`: failed byte = 0 now documented as *[choice]* (native: stale stack slot,
  address-dependent, unspecified). `RzNullIStream::new_null` comment: flag false is spec fact (OS6).
- `text.rs` `format_g` comment: spec model (17 correctly rounded digits, half-up at 7th), Rust `{:.16e}` used as a
  generic correctly-rounded formatter at the C-runtime delegation boundary; OS7 residual a *[compat-limit]*.
- `tests.rs`: test 14 exact-fit now asserts no Skip; short_reads GetGZStr expects 300 bytes; new `t15_float_ties`
  (all §2.2 vectors), `t16_failed_open_flush` (state FAILBIT only, binary and text), `t17_short_reads`,
  `t17_length_failure_call_count`.

**Audit, per owner (complete vs remaining):**
- `StreamCore` (§1.1/§1.6: count, Release <=1 quirk, QI IIDs, SetUserData counted assignment, Init/Shutdown,
  destructor release): complete.
- `RzOStream` (§1.3-1.5, slots 3-18): complete. `RzNullOStream` (§3): complete.
- `RzTextOStream` (§2): integers, strings, hex dump, GetError true: complete; `%g` complete to the spec model,
  OS7 residual open (no credit claimed for unknown runtime formatting edges); locale decimal point compat-limit.
- `RzOFStream` (§4): open modes, sticky bad/fail, Flush on failed open (OS4), binary SetRZCharStr raw, autoFlush:
  complete; OS1/OS2 (log contents, text-mode CRLF/locale) open, unreachable in 1.1.641.
- `RzIStream`/`RzNullIStream` (§5): complete for every specified behaviour; compat-limits: decoder >8-byte overrun
  (consumed and ignored), failed-byte value, first-chunk stale bytes, C buffer overrun (dropped bytes).

**Gate:** qbuild all PASS: cargo test 4103 passed/0 failed, clippy 0, ctest 65/65. fmt clean on touched files.
**Game:** none; no plugin or CAPI code uses rz_streams, so no shadow exists.
**Questions:** none filed.
