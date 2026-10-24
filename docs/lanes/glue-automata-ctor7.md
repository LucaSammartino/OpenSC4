| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-ctor7 (owners capture references and construction codec) | Codex / glue-automata-ctor7, detached HEAD | READY measured capture hardening: Rust 6933/0, clippy clean, CTest 76/76; three house turbo runs to 2032, owners 219/219 exact, ref_warnings=0, foreign_exceptions=0; construction 563/563 exact in these runs; LR13 open | Supervisor publish detached commits; review/land measured work and credit coverage; resolve LR13 before claiming the original construction failure fixed. |

## Changes and ownership audit

582d19fda stores each capture-owned serializable pin explicitly rather than inferring ownership from the capture-wide retention flag. It separately counts every replay AddRef by capture ID. Replay releases require an outstanding acquisition; an unmatched release is rejected and logged instead of consuming the capture pin or a game reference. ClearReferences drains only recorded replay acquisitions and explicitly retained pins, including cleanup after a partial record. Move operations transfer both ownership records without adding or releasing references.

The audited ordinary failure paths remain balanced: Capture releases its root serializable QI even when Write returns false; failed serializable QI does not register a reference; duplicate incoming pointers reuse their registration; the grid identity QI is released immediately; the variant object getter's successful QI is released after capture even when capture fails; replay IID QI is released immediately and the token's separate AddRef is recorded. Failed Rust reads release their tracked input acquisitions; successful object reads release their output references through the guarded ledger, with abandoned acquisitions reclaimed at capture destruction. Construction roundtrip releases every tracked input token on both success and failure.

The actual DLL capture seam additionally tests zero-debt release rejection, repeated acquisitions, invalid 64-bit IDs, truncated input, and failed native Write. Its previous adjusted-interface, writer-released temporary, duplicate/null, moved-from destruction, unknown-ID and class-collision coverage remains passing. Replay now logs the byte offset, class and requested IID when a non-null reference does not answer the reader interface.

## Construction limit and question

**LR13**, appended to [questions/lot-records.md](../specs/questions/lot-records.md), asks for the native writer-facing pointer identity, QI ownership and temporary lifetime contract of construction references, especially the effect views. The train180 254/250-byte record and bare-occupant last-reference warning were not reproduced in these three runs. No construction codec behavior was guessed or changed, and the literal documented IID checks remain. The original failure's root cause is therefore still open; this is a READY entry for the implemented and measured capture hardening, not a claim that the historical codec defect has been resolved.

The owners shadow remains default-off and is enabled only by its existing INI key in these requests. No tools files, coverage maps, ABI exports or ABI version changed. No main merge or branch/ref update was performed.

## Gates

| Gate | Terminal result |
|---|---|
| qbuild all; qbuild-20261024-054455-build-ctest | PASS: Rust 6933 passed/0 failed/0 ignored, clippy diagnostics=0, native build 245/245, CTest 76/76 |
| Required queued qbuild verify; qbuild-20261024-054910-build-ctest | PASS: native build 46/46, CTest 76/76 |

Both original asynchronous operations were collected to terminal PASS, without duplicate checks. Engine source stayed unchanged after 582d19fda. Full logs and native verdicts remain in build-runs/, including qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log and build-results/.

## Real-city measurements

All requests used the switches from main/build-runs/regress-train180-shadows.ini, including both automata shadows and the other house shadows, with turbo=1, end_date=2032-01-01 and seconds=1800. Each completed at 2032-01-01 with ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0 and overruns=0. No owner roundtrip failures, unavailable interfaces, rejected replay releases or other WARNING lines occurred. Each shadow was exact on all 72 months plus load: 219/219 samples per shadow overall. Construction records were 563/563 exact overall. No turbo-only difference was observed.

Exact owner clean/total lines and run records:

| Line | Run | Construction exact | Real seconds |
|---|---|---|---|
| ctor7-house-2032-20261024-a1 73/73 | [423](../runs/ctor7-house-2032-20261024-a1.md) | 183/183 | 612.515 |
| ctor7-house-2032-20261024-b1 73/73 | [424](../runs/ctor7-house-2032-20261024-b1.md) | 189/189 | 749.156 |
| ctor7-house-2032-20261024-c1 73/73 | [425](../runs/ctor7-house-2032-20261024-c1.md) | 191/191 | 707.250 |

The same three lines also apply independently to automata_shadow, each 73/73. Full logs and verdicts are in build-runs/game-results/ under these exact labels. Every unique request used one event-based blocking wait; completed verdicts were accepted and no duplicate request or qwait was used. No requests remain outstanding.

Run a1 used the full-gate DLL (harness SHA256 prefix b1817bc5); the separately required verify rebuilt the same engine source, and b1/c1 used SHA256 a31aa558b0cd48603685d474478c4fd65c715499de038ae9e3c21e0a80e326f1. This distinction is preserved in the run records.

## Next steps

The supervisor should publish the detached commits to the reserved lane branch, review and land the READY measured work, and apply coverage credit from these logs. LR13 requires a spec answer before implementing or claiming a fix for the historical construction mismatch. The coordinator owns any house-profile or coverage-map changes; none were edited here.
