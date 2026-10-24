| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-viewutil-vu16 (native view utility ABI) | Codex / `re-viewutil-vu16` | Ready: spec `6556a2c1f`; VU16/VU15 answered; 50 native call contracts, 4 new vectors; doccheck and questions check PASS; ABI unchanged | Land with glue-viewutil-shadow11, retain archived VU16; implementer can extend native comparisons under §13; VU12/VU13 remain open |

## Report

The new [view-utilities §13](../specs/view-utilities.md#13-callable-native-contract-and-shadow-isolation-vu16)
provides callable machine-interface declarations for every entry in the 50-body family: conventions,
receiver placement, ordered four-byte argument slots, pointer layouts, stack cleanup, AL/EAX results
and eight x87 scalar returns. Min/max take four float32 arguments with cdecl cleanup. The angle
helper and tolerant ground-plane wrapper are receiver-independent callee-pop entries. Float32 and
float64 ST(0) captures are explicitly distinguished from a claim about the original C++ return type.

Storage rules separate live owner reads from local mutable arguments, lazy owner writes, iterator
mutation and renderer/camera/model/instance providers. Copying a view owner does not isolate its
borrowed renderer. Executable identity, SHA-256, preferred-base/RVA binding and unsupported rebasing
limits are specified. No public ABI rows or exports are proposed; no ABI version changes.

VU16 was imported verbatim from glue-viewutil-shadow11 before answering, checkpoint `9e5892ad6`.
Its report has no separate Owner closure table; its explicit remaining-work rows say 1/50 entries
compared and 49 blocked. §13 supplies the missing contracts without upgrading that measurement.
The block input-width evidence also closes VU15: native scalar inputs are float32; §11.18 now labels
INT_MAX as an integer-stage vector. New vectors 22–25 cover stack frames, min/max NaNs and signed
zero, representable block inputs, scalar captures, receiver-independent calls, binding and isolation.
The spec/gate milestone is `6556a2c1f`.

Measured evidence: **5444 exported instructions across the 50 bodies match the examined executable,
zero byte mismatches**. PE32 i386 preferred base `0x400000`, SizeOfImage `0x7C8000`, timestamp
`0x4C12BFB7`. This is static byte verification, not executed numerical or whole-owner conformance.
Queries used main-checkout re_query.py and narrowly selected decomp sections; no REA was needed.
Ignored evidence helpers: `build-runs/inspect-vu16.py`, `build-runs/evidence-vu16.py`.

Validation: questions index regenerated; `python tools/questions.py check` PASS;
`python tools/doccheck.py check` PASS; `git diff --check` PASS. Logs are
`build-runs/questions-index-vu16.log`, `build-runs/questions-check-vu16.log`,
`build-runs/doccheck-vu16.log`. Cargo, clippy and CTest: n/a (spec-only work).
Game runs: none requested or performed; shadow exactness: unmeasured by this lane.
No engine, plugin, capi, shared ABI tables, coverage-map, README or coordinator Handoff edits.

Questions: **VU15 and VU16 answered** in
[the archive](../specs/questions/answered/view-utilities.md), index regenerated; none filed.
VU12 (numeric mode/store graph) and VU13 (generic decoder extremes) remain open.
The train must reconcile the incoming open VU16 from glue-viewutil-shadow11 with this archived
answer: retain the archive, drop the incoming open copy, then regenerate the question index.

## Handoff

- Ready for coordinator publication from actual detached HEAD; queue uses kind re and its full hash.
- Next: land both lanes with the VU16 question reconciliation above; implementer may extend comparisons using §13.
- No game request, gate process or unfinished native experiment is outstanding.
- Remaining numeric questions are VU12/VU13; no whole-owner implementation/verification credit is claimed.
- q.py Bash wrapper failed with CreateFileMapping denial; local PowerShell tools worked. Shared packed-refs lock attempts were denied during commits, but detached HEAD commits succeeded and shared refs were not changed.
