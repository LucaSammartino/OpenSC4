# Reach tracer exclusions and diagnostics

The 1.1.641 function roster contains false entries inside instruction operands.
An entry breakpoint at such an address changes the surrounding instruction;
the CPU need not execute an INT3. Function names and a plausible first byte do
not establish an instruction boundary. Do not infer safety from alignment.

`FC4Reach.cpp` has an explicit never-arm list for the confirmed artifacts below.
It applies to preferred image base 00400000, independently of the relocated
load address. Every excluded row retains zero counts and its reason, and is
also listed in the TSV's comment header. Reach remains a lower bound.

| Roster address | Containing instruction address | Operand that must remain intact |
|---|---|---|
| 00420036 | 00420034 | QueryInterface IID immediate |
| 004f4442 | 004f4440 | Relative call displacement |
| 00505553 | 00505552 | Position getter store ModRM |
| 00544547 | 00544546 | Pedestrian caller stack-store ModRM |
| 005c3a63 | 005c3a62 | Stack-load ModRM |
| 005f5a52 | 005f5a50 | Adjustor thunk displacement |
| 00656c64 | 00656c61 | Absolute global address |
| 00682632 | 0068262f | Spinner stack-store displacement |
| 00736559 | 00736558 | Stack-test ModRM |
| 00786f62 | 00786f60 | Budget UI stack-store SIB |

Evidence was obtained with `python tools/re_query.py asm ADDRESS --bytes`,
paging enclosing functions and decoding the omitted ranges from the adjacent
instruction boundaries. The list records operands, not copied code. Existing
runtime, unsafe-opcode and hook exclusions remain separately identified.

The disaster reproducer is Berlin / Konradshohe with
`engine/tests/scenarios/scripts/berlin-konradshohe.script`, turbo, shadows and
end 2026-04-29. Before the fix, `reach-fix28-diag3-disaster` reproduced the
00508317 access violation reading address 5, thread 12088, after 1,116,454
entry/step events. At the fault no step was pending, previousTF was zero,
EFLAGS was 00010206 (TF clear), DR6 ffff0ff0 (no hardware breakpoint), and
EDI was 5. The ring's last entry/step pair was 007fe190; its step completed.
The corrupted stack write at 00544546 explains the bad argument to 005080c0.
There was no intervening foreign exception in the diagnostics. This failure
is a patched operand, not a swallowed game exception or copied/self-read code.
The matching grown-city baseline exited C0000005 with no diagnostic report;
do not claim its exact fault context from the disaster evidence.

For investigations only, `reach_diagnostics = <absolute file>` opens a dump
before arming. It is off by default and requires `reach = 1`. Each thread
retains its last 32 entry/step register contexts. At most 16 foreign exceptions
dump the current thread's ring, pending entry and prior TF ownership. Ordinary
traps do no file I/O. Delivery to other handlers is unchanged. The file can be
empty when no foreign exception reaches the tracer; that is not a clean verdict.

Native `reach_test` covers all ten exclusions using a relocated synthetic image:
patching a bogus entry would change a return immediate without raising INT3.
It checks unchanged execution, a valid neighboring entry, and all ten header
records, alongside existing concurrent, WoW64, flags, x87 and SEH tests.
