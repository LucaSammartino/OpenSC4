# S3 round 3: marginal adaptation measurement

Start: 2026-10-25T13:36:50Z, written in the lane file before investigation.
All elapsed time after this start is charged, including the failed PowerShell
and Bash-wrapper attempts, the 323-second focused compile, full gate and game waits.
Prior completed work is unchanged; round-2 view-utilities Rust is untouched.

The ten new selections are in functions.csv. The entry ABI comes from their
exact rows in docs/abi/vtables.tsv. Behaviours: lighting §§1.2,4.4,4.6 and
strings-language §§4.3,4.4,4.6. No next-entry spacing is used as a body size.
None of these ten has a published body-byte count in those interface/spec
sections; zero is an uncredited lower bound, not a zero-length function.
Consequently the requested 8 KB cohort and 11 KB/hour positive result cannot
be established with this cohort, even if comparisons are exact. No coverage
credit is requested. Sizes and complete entry contracts for the larger rejected
functions would decide whether a sufficiently large cohort can pass.

New transport: native lighting offsets (night +0x24, shadow +0x64, global +0x8c,
axes +0xe4), counted byte-strided normals/BGRA, row-major 3x3 matrix, and EAX/AL
returns. No round-2 view word packet is reused. Lighting inputs import into the
existing production model; the existing language manager performs mappings.
Current-ID input zero is refused where the manager's field offsets are absent.
Vertex streams are limited to 1..4096 entries with valid bounded strides and
non-overlapping output. Refusals remain in the evidence and qualification test.

The facility snapshots each containing committed writable private region, at
most 64 MiB, before and after native execution. It detects net changes outside
the declared receiver/input/output windows in these regions; it cannot observe
undone writes, other allocations, image globals or the active stack below the
caller's arguments. Concurrent writers are not attributed. These are signals,
not a proof of whole-process confinement. Aliases and invalid regions refuse.

Replay uses isolated copies after the native interval. It never overwrites live
native state. Receiver/input byte changes are compared too. Every injected XOR
changes one byte of one expected output (or one scalar return bit), then uses
the same comparator and logs its window and byte offset; window 6 is EAX/AL.
Memory output integer/float tags determine whether all differences are within
four ULP; BGRA and language returns are integers, never float-rounding matches.
All additional behaviour is behind the existing default-off inplace settings.
