| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-stream-core-answers4: output-streams OS3-OS6 (float ties, failed-open flush, failed byte/chunk, Skip(0), null-input flag) | `re-stream-core-answers4` / (RE lane) | ready, docs only, no ABI change; OS3, OS4, OS6b MATCH; OS5a/b, OS6a DIFFER; OS7 filed | 1. core streams: GetRZCharStr no Skip(0); GetGZStr append failed chunk from one reused zero-initialised buffer; conformance §9 tests 15-17. 2. OS7 (runtime %g sweep or ld12 emulation). |

## Report
Spec-only lane, static analysis (tools/re_query.py asm, decomp/lib.c); REA not needed. Commits 4e8c9c0a (proof facts), 1afc8966 (spec + questions).

- OS3 (§2.2) MATCHES: %g = double rounding, half-up both times. `__fltout2` 0x00A0277B asks `$I10_OUTPUT` 0x00A04942 for 17 digits (18 generated, half-up on digit 18); `__fptostr` 0x00A0264A rounds half-up on the 7th. Ties go away from zero; 0.1234565 (stored below the tie) prints 0.123457 (correctly rounded %g would give 0.123456). Proof that exact 17-digit ties cannot reach the 6-digit output; the residual case (scaling error of 12-byte multiply) is OS7.
- OS4 (§4.2) MATCHES: failed-open Flush leaves the state unchanged (filebuf sync 0x0092BF91 returns 0 outside output mode; output mode set only by 0x0092C327 on an open file). New quirk: null ofstream pointer makes Flush fault.
- OS5 (§5) DIFFERS: (a) length decoder 0x0092B85F ignores read results; unwritten slots hold the bytes of the stream's own address (prologue PUSH ECX x2), so the outcome is unspecified natively; 0 kept as a [choice] that is one native outcome. (b) GetGZStr 0x0092B92D appends a failed chunk before testing the error; later chunks carry the previous chunk's bytes (specified), first-chunk stale bytes unspecified (choice: 0).
- OS6 (§5): (a) DIFFERS, Skip only when length - k != 0; (b) MATCHES, ctor 0x0092BC80 sets flag false inline.
- §7 quirks 14-16, §8 OS7, §9 tests 15-17 (float ties, failed-open flush, short reads). Questions archived in docs/specs/questions/answered/output-streams.md (new file); QUESTIONS.md regenerated.
- Engine conformance: the current core behaviour differs from native on OS5b (no append) and OS6a (Skip(0) call); OS5a choice stays.
