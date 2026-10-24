| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-shadow13: EF23 capture and EF22 scope | Codex / `glue-effects-shadow13` (detached HEAD, engine lane) | ready partial: merged re-effects-ef24 ee82b9c36; cargo 7408/0/0, Clippy clean, CTest 81/81; runs 631 and 632 clean, effects 73/73 each, ref_warnings=0; ABI unchanged; no new questions | 1. Publish reserved commits. 2. Credit only measured name membership. 3. Supply EF22 host capabilities before broader observation; coordinator reconcile effects_shadow in current shadows profile. |

## Changes and coverage boundary

Merged `re-effects-ef24` at `ee82b9c36` and built on glue-effects-shadow7's
73/73 name-membership measurement. Implementation `db988ae7a` captures EF23's
complete sequence extent through D+1B and retained u32 flags at D+18 for named
and anonymous descriptions. Capture preserves high bits; stream Read's mask
does not apply. Truncated objects refuse complete payload support. Two focused
tests cover extent boundaries, empty vectors, names and unmasked flags.
`6a1824077` updates the existing EF21-8 vector to assert complete support and
retained flags 80000005 after EF23's answer.

The default-off `effects_shadow` now labels its dated measurements explicitly
as `scope=name-membership`, and separately reports EF22 attachment refusal and
unavailable payload/history. Name comparisons use the existing SDK manager
getter and Rust directory parsing; native answers do not seed or repair core
state. No C ABI export changed and no ABI version was bumped.

**This is ready partial work, not whole-manager/resource verification.** EF22
section 18.8.6 provides a guarded attachment recipe, not an observer API. This
host lacks pre-Init attachment, exception-transparent gateways, and exclusion
covering every writer. The spec requires refusal when those capabilities are
missing. No hooks, executable probes, guessed signatures or live payload reads
were added. Retained resource fields, sequence state, active instances and
native history remain unavailable and earn no coverage credit. The EF23 tests
are synthetic conformance evidence, not native sequence observation.

No new question was filed: EF22 and EF23 are answered, and EF22's closed host
refusal is explicit. No answered question was duplicated.

## Validation

Final engine changes are committed through `6a1824077`; later commits record
evidence. Corrected `qbuild all` completed **PASS** on stable final source:

- Cargo **7408 passed, 0 failed, 0 ignored**, summed across all result lines.
- Clippy **0 diagnostics**, with `-D warnings`.
- Queued native operation `qbuild-20261024-212917-build-ctest`: build
  **209/209** edges (8.5 s), CTest **81/81** (116.2 s), overall **126.9 s**.
- Focused capture tests **8/8**; owned Rust formatting **2 files, 0 diff lines**.
  Corrected vector's focused turbo-off suite **1/1** (2.07 s).

The first full gate failed on the old EF21-8 `supported=false` expectation;
its Rust logs are preserved in `build-runs/gate-r1-failed/`. That gate reported
993 passed/1 failed, Clippy clean, native build 264/264 and CTest 81/81 under
`qbuild-20261024-203520-build-ctest`. The diagnosed vector correction justified
the second full gate. Final logs remain in `build-runs/qbuild-*.log`,
`fc4verify-*.log` and the unique `build-results/*.verdict` files.

## Real-city evidence

Final counted line:

```text
effects13-names-2032-20261024r2b 73/73
```

[Run 632](../runs/effects13-names-2032-20261024r2b-661.md) tests the final
queued build's DLL, SHA256
`A31928046E28EF21587535861C20FEF840DB8A187938D755B2699CFE48E8C534`.
Konradshohe, turbo through 2032-01-01: **clean**, `ref_warnings=0`, all other
problem counts zero. All 73 dated name-membership lines have `refusal=0`,
`resyncs=0`, `mismatches=0`; first 2026-01-27 load, last 2032-01-01 month.
First and last samples each capture 1,094,484 bytes, 1,154 entries and 3,465
queries. Terminal date 2032-01-21, 28.828 s city runtime, slot 1, wall 42 s.
Each sample explicitly retains `payload_unavailable=1 history_unavailable=1
attachment_refused=1`; these unavailable fields are not exact comparisons.

[Run 631](../runs/effects13-names-2032-20261024r2a-660.md) separately records
the earlier DLL: `effects13-names-2032-20261024r2a 73/73`, clean,
`ref_warnings=0`, 28.407 s runtime. The final build changed the DLL hash, so
run 632 tested that exact artifact. No normal-speed rerun was needed because
neither run had a difference. Full game logs and verdicts remain under
`build-runs/game-results/`.

Both blocking filesystem watchers missed the already completed verdict files
and were collected to terminal timeout; the valid unique clean verdicts were
retained without resending. No build or game operation remains outstanding.

## Next steps

The coordinator can publish this ready partial work and reconcile
`effects_shadow` with the current single shadows-profile string after review.
No tools files, coverage tables or shared refs were read or edited here.
Broader observation requires implementing and validating EF22's missing host
attachment/exclusion capabilities before enabling native payload capture.
