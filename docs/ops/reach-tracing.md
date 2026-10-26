# Reach tracer exclusions and diagnostics

## Opt-in city lifecycle census

`reach_lifecycle = 1` (default **0**, requires `reach = 1`) arms entries at the
end of `PostAppInit`, after plugin setup, instead of after `PostCityInit`.
It captures real city deserialization, initialization and native teardown.
The observer's load/shutdown comparison passes suspend tracing, as monthly
constructed probes already do. Entries changed by a late hook remain excluded.
The first window's `2000-01-01` is explicitly a **load sentinel**, not a saved
city date; `PostCityInit` opens the actual date window. It is unsuitable for
calendar-based scenario shortening. Post-city-shutdown writes a checkpoint
without stopping; `PreAppShutdown` stops before plugin hooks are restored.
No settings or city state change merely by enabling this tracing option.

First generate a reduced roster with `python tools/suite_reach_gaps.py
--baseline build-runs/suite/BEFORE --out build-runs/gaps
--contracts build-runs/exact-contracts.tsv`. Lifecycle tracing of the complete
roster is unqualified: reach29-r1 had three corrupt runs and one clean run.
Use the generated roster explicitly as `reach_contracts = <absolute path>`;
it retains reached and unreached implemented functions under baseline-exact
keys. It changes the instrumentation footprint, never the coverage weights.
The earlier reduced-roster r2 had three corrupt mature-city runs. The
continuation found and excluded the false entry at 00707865 (below). The
rebuilt-DLL `reach29-repaired` replay qualifies all four rows: 4/4 clean and
end-reaching, 47/47 actions, zero aborted steps or thread-slot exhaustions.
The complete 35,648-entry roster remains unqualified. The lotdev and pollution
mismatches in West Weburg also occur with lifecycle off and are separate engine
findings; lifecycle qualification does not mean every shadow is exact.
`python tools/suite.py run --set reach29 --profile shadows --slots 4
--extra "reach = 1" --extra "reach_lifecycle = 1" --extra "reach_cap = 1"
--extra "reach_contracts = <absolute path>" --tag TAG` runs the four load/save,
empty-development and Lua-view scenarios in isolated queue slots. Add
`--scenario NAME` to select a single qualified row. `--reach` may replace the
first extra setting; explicit roster extras take precedence over its defaults.
`python tools/suite_reach_gaps.py --baseline build-runs/suite/BEFORE
--after build-runs/suite/NEW --out build-runs/reach-delta` ranks exact-shadow
gaps and reports per-owner executed/verified gains and first mismatch lines.
Both measurements use the same current byte attribution and the conservative
suite-wide exact-key rule. Capturing a lifecycle does not broaden a shadow's
documented comparison scope.

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
| 00707865 | 00707864 | Tract Init absolute global address (00b43d94 becomes 00b43dcc when patched) |
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
Arming 00544547 changes the stack store at 00544546 into a register move
that sets ESP from ECX, so the callee uses object storage as its stack.
Fault ESP=1285e240 is exactly EBP=1285e2c0 minus the resulting frame size,
0x80 bytes. This explains both the bad argument and the apparent alternate stack.
There was no intervening foreign exception in the diagnostics. This failure
is a patched operand, not a swallowed game exception or copied/self-read code.
The matching grown-city baseline exited C0000005 with no diagnostic report;
do not claim its exact fault context from the disaster evidence.

The reachgen29 continuation isolated mature-city lifecycle corruption using
four queued roster splits of 3,146, 787, 196 and 49 entries. The only failing
range at each split contained 00707865. `re_query.py asm 00707864 --max 4`
decodes the real instruction as `mov eax, dword ptr [0xb43d94]`; the purported
entry at 00707865 is its address operand. Replacing 94 with CC reads a different
global, stores it at tract developer +30, and faults later during lot growth.
Old-DLL controls omitting just that entry reached the end date in all four
scenarios: three clean, one empty-city stalled exit (19/19 actions). These are
diagnostic controls, separate from the rebuilt-DLL qualification results.
`--reach` now supplies defaults before explicit `--extra` values, so the
selected `reach_contracts` roster is preserved; the first exploratory split
used the full roster because the earlier CLI silently overwrote that setting.

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

Validation: `build-runs/acceptance.py` checked twelve clean, end-reaching runs:
on/off pairs of the disaster, the idle house census, and the original luacmd27
grown-city fire, garbage, health and power scripts (source equality checked).
All six on-runs wrote a TSV with ten exclusions; all eight Lua runs completed
and paired lot statuses agreed. Shadows were 5747/5988 exact, not all exact.
Health/power pairs used the existing FC4_WATCHDOG_STALL_MS and
FC4_WATCHDOG_NOTICK_MS environment settings at 60000, with the 650-second hard
harness limit retained; the other accepted pairs used default watchdog limits.
The default ten-second watchdog also terminated some reach-off Lua baselines,
including landmarks, after completed scripts but before calendar advancement.
Those failures remain separate; this fix does not remove that harness limit.
An extended-limit power-off attempt timed out after a slow startup; its
isolated r3 retry completed cleanly. See docs/lanes/reach-fix28.md for labels.
