| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-span565-context4 | Codex / `verify-span565-context4` (detached, supervisor publishes reserved branch) | ready: cargo 4938/0, clippy clean, ctest68/68; final own/current40+candidate clean2032/ref0; constructed native249/249 exact each | Merge verification changes; retain opt-in and existing open questions; no natural-use or owner-replacement credit |

## Report (2026-10-22)

Replaced the old raw-context-field/direct-blit fixture with the published public factory,
Init, setter and slot-22 producer route. Safe acquisition/calling/lifetime feasibility was
established within the initial 15-call bound; substantive code preceded call30. No private
context or buffer fields are inspected or written. Public vtables are checked against the
published contract before calling them.

Real-work commits: `5a1cac9f`, `02b46fdc`; public documentation update `eac50cb6`.
Implementation is confined to `FC4Span565Bridge.cpp`, new private includes
`FC4Span565Measure.inc`/`FC4Span565Fixture.inc`, and `FC4Span565Bridge.h`.
No Rust pixel math, C ABI, ABI version, defaults, maps, shared registration lists or tools changed.

## Independent comparison and measured scope

- Original native methods mutate the live target exactly once per slot-22 call. Rust receives
  independent guarded copies of the complete padded preimage, source allocation, source/blit
  records, spans and gradient input captured before native execution. Native output remains
  authoritative. No second host draw, second-Rust oracle, output copying, live restoration or resync.
- Destination: native type5, 17x12, stride36, all432 bytes compared per call. Guards cover native
  padding, rows and pixels outside the spans; Rust destination/source copies also have32-byte
  outer canaries. Overlapping spans and an empty span exercise processing order and boundaries.
- Each run compares249 cases:247 require changed native output, plus2 explicitly separated
  zero-alpha controls. Total107568 compared bytes and3257 changed native bytes per run.
- All156 publicly reachable row/kind pairs are covered using native source types2/4/5/6/7/8/9,
  public key setters, plain, both stretch orientations (2,.5)/(.5,2), tiled, alpha and modulation.
  Source type8 is constructed with32-bit texels, as in the sibling fixture architecture.
  Context-forced unkeyed type2 is distinguished from keyed source types; unreachable row13 is
  excluded rather than synthesized through private state or credited as context coverage.
- Other vectors exercise signed alpha values0/2/-1/INT_MIN, low-byte modulation0x100/2,
  varying and zero gradients, null-gradient clearing, and null/refusing source setup. All six
  source-record dwords and defined format/key/geometry members are checked at renderer entry.
  Inactive geometry/key fields and stack padding are captured verbatim, without invented values.
- Public reference probes observe target1->2->1 and source1->2->1, with created-object final
  Release0. IsLocked restoration is checked after every textured draw, plus idempotent
  Suspend/Resume and Shutdown-after-Suspend. No private lock-word/flag inspection or SR565-6
  failure-exit policy change. Sentinel output rectangles and refusing-source QI/ref counts are checked.
- Exact CALL return addresses classify constructed native, constructed direct, natural native,
  and other calls separately. Each run has constructed slot counts16/3/3/3/224; site counts
  solid16, gradient3, alpha3, alpha-gradient3, plain56, stretch112, tiled56. Direct/natural/other
  counts are all0; these are absence observations, not 0/0 algorithm-exactness claims.

Every measured run reports249 exact, different0, refused0, fallback0, resync0, unaccounted0,
unexpected-unchanged0, contract-failures0. Seven source lifetimes complete with Release0;
all five patched renderer entries are restored at graceful shutdown.

## Gates and real-game evidence

The first build included substantive code, as required by the brief. Baseline `qbuild all`:
cargo4938/0, clippy0 diagnostics, C++208/208, ctest68/68; its tree-changed marker is recorded.
The stable final `qbuild all` at `eac50cb6` passed cargo4938/0, clippy0 diagnostics,
C++6/6 and ctest68/68 (75.2s), fc4verify PASS (78.9s), with no tree-changed marker.
Later commits change only this report. No .rs files changed, so a rustfmt file check is inapplicable.

| Immutable request label | DLL | Verdict/date | 565 result | Save/shutdown |
|---|---|---|---|---|
| span565-context4-r1a-own | r1a | clean2032-01-12/ref0 | 249/249 exact,156/156 pairs | full save ok1, hooks5/5 |
| span565-context4-r1b-own | final r1b | clean2032-01-06/ref0 | 249/249 exact,156/156 pairs | full save ok1, hooks5/5 |
| span565-context4-r1b-current40 | final r1b, all40 supplied keys+span565 | clean2032-01-01/ref0 | 249/249 exact,156/156 pairs | full save ok1, hooks5/5 |

Final immutable DLL: `build-runs/dll/span565-context4-r1b/FreeCity4.dll`, SHA256
`6600F8627A2848C1053ACFC08F68CDF95CE0B1D085F7A4DDF73F936485FB24BF`.
Baseline r1a SHA256: `9364588BBA0212E7975CC6982C6AD53D8AE8947F839F118446C1BE51BC4CCEA2`.
The stable gate rebuilt header dependencies, so final own/union runs used a separately frozen
r1b instead of overwriting r1a. Requests, logs, verdicts, script and DLL/PDB copies remain under
this worktree's `build-runs/`. No requests are outstanding. All labels are unique; none was discarded.

Both final runs freshly load the ordinary city, save through the public game interface and quit
through shutdown. The constructed context is transient and destroyed before city load; this change
introduces no serialized renderer state and claims no saved-city reload compatibility. Normal-speed
fallback testing was unnecessary because no turbo-only differences occurred.

## Questions and remaining limits

No new questions. Existing SR565-1/5 natural-use questions and SR565-6 lock-policy question remain;
this lane does not close SR565-2/3/4 from its bounded constructed inputs. No unsupported format was
forced into normal game drawing. No default promotion, whole-owner replacement or natural-path
algorithm exactness is claimed. Keep span565 opt-in and retain the published unresolved questions.

## Coordinator measured-shadow audit (2026-10-22)

span565_source_own 249/249; span565_source_current40 249/249; span565_joint97 249/249.
Actual original context route versus production Rust pixel CAPI, full107568byte arenas,
3257bytes changed by native,156/156 reachable pairs. Joint train97 DLL8a7160ca,
current40+span565, clean2032/ref0, fullsave and shutdown5/5entries restored;
zero mismatches/refusals/fallback/resync/contract failures. Source final DLL6600f862.
This qualifies the measured same-process shadow under COVERAGE.md, as for1555/4444.
Natural renderer calls remain0; SR565-1/5/6 and unexercised cases remain.
No natural-use/full-owner exhaustive proof or replacement/default promotion.
Raw joint evidence/hash: build-runs/run4-train97-native-audit.json.
