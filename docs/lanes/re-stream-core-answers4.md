| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-stream-core-answers4: output-streams OS3-OS6 (float ties, failed-open flush, failed byte/chunk, Skip(0), null-input flag) | `re-stream-core-answers4` / (RE lane) | in progress | see Handoff |

## Handoff
- Goal: answer OS3-OS6 (docs/specs/questions/output-streams.md) from native bodies; write rules into output-streams.md §2.2, §4.2, §5; archive to answered/.
- Done: lane file; all proofs read (static, re_query asm + decomp/lib.c; REA not needed).
- Proof facts: OS3 MATCHES: _fltout2 0x00A0277B passes 17 (PUSH 0x11) to $I10_OUTPUT 0x00A04942 (18 digits from 80-bit-mantissa scaling, half-up on digit 18); __fptostr 0x00A0264A half-up (next digit > '4') to 6. 0.1234565 stored -> "0.123457" (double rounding), 1234565 -> 1.23457e+006, 100000.5 -> 100001, 12345.25 -> 12345.3.
- OS4 MATCHES: Flush 0x0092BCF7 -> 0x0092BA02 rdbuf()->sync (filebuf vtbl 0x00AE4D6C +0x10 = 0x0092BF91) returns 0 unless out-mode byte +0x68 set; only switch-to-output 0x0092C327 sets it and needs is_open +0x60. Null ofstream ptr -> Flush faults.
- OS5a DIFFERS: decoder 0x0092B85F ignores GetUint8 result; buffer = 8 stack bytes pre-filled by PUSH ECX x2 with `this` (bytes of the stream's own address), so unwritten byte = address byte, never "previous". OS5b DIFFERS: GetGZStr 0x0092B92D appends chunk (Append 0x0090F053, counted) BEFORE GetError check; buffer stale (first chunk: uninit stack; later: previous chunk bytes).
- OS6a DIFFERS: Skip only if length-k != 0 (JZ 0x0092B908). OS6b MATCHES: ctor 0x0092BC80 inline, flag +8 := 0.
- Next: write rules into output-streams.md §2.2/§4.2/§5/§7/§9, move OS3-6 to answered/, questions.py index.
