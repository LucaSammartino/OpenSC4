| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span4444-context4 (native context and packed blend) | `glue-span4444-context4` / engine lane, detached reserved worktree | ready: implementation b238183b; cargo 4175/0, clippy clean, ctest 67/67; native and literal union clean through 2032/ref0; constructed native 476/476 exact; shipped 0; ABI unchanged; SP7 filed | Integrate supported opt-in changes; regenerate question index; resolve SP2/SP4/SP7 before broader/default/full-class credit. |

## Report

Reserved and used the final one logical-family round for this phase. Reconciled only the newly published
4444 SP1/SP6 contracts; no sibling renderer behavior, factory replacement, ABI exports or ABI version changed.

### Supported changes

- `f793142b`: supported core correction committed before call 30. Section 6.1 now uses the shared wrapping
  red/blue dword, separate green intermediate, logical shifts and final masks. Alpha gradients retain the
  full unchecked wrapping domain. Added 22 independent published expected-word vectors (all weights 17..31,
  boundary/carry/borrow cases) and two discriminating weight/wrap ramps in `6a31339f`.
- SP6 applicability is explicit in 4444 sections 1.3/1.3.1: Init selects renderer vtable `00AC185C` for type7,
  while the setters and draw consumer remain shared. No contract was inferred from the separate 3D context.
- `6a31339f`, `8e00ae2e`: real buffer/context route using the existing `FC4SpanContext` creation, native-slot
  and lock helpers, with 4444-only cases in `FC4Span4444Context.inc`. Exercises writable output rectangles,
  nonorigin placement, quantized forward/reciprocal scales, tie rounding, source-edge tiling, tiling precedence,
  native transparency setters, null-source and failed-QI no-write contracts, source locks and public reference
  probes. Classifier uses exactly site+3 for 4444, with 56 positive/negative checks; sibling classification stays
  unchanged. The bridge observes actual native format/key/mode descriptors and counts shadow refusals too.
- `b238183b`: corrected native source acquisition in `FC4Span4444Source.inc`. A plain context may bias the
  source pointer before the allocation; `stride*height` from that pointer is not an allocation boundary.
  Stage only the texels actually sampled by section 9.1, preserving native coordinate slots for the existing
  C ABI. This reads texture inputs, never original same-owner renderer outputs. Overlapping source/target
  regions are explicitly refused and counted pending SP7; the bridge remains opt-in and partial.

### Gate

Baseline `qbuild all`: cargo 4173/0, clippy clean, build 205/205, ctest 67/67; reported changed tree during work,
so this was baseline evidence only. A transient quick failure was an overflowing test literal type, corrected.
Latest quick: core 2318/0, dependents compiled, build 4/4 PASS. Touched Rust files pass the required wrapper's
rustfmt check. Final committed-implementation `qbuild all`: cargo **4175/0**, clippy **0 diagnostics**, build
**4/4**, ctest **67/67**, PASS in 81.8 seconds with no changed-tree warning. Logs are in this worktree's
`build-runs/qbuild-{test,clippy}.log` and `fc4verify-{build,ctest}.log`. `git diff --check` passes.

### Real game evidence

Each request used a unique label and one local Python wait, without qwait. All completed verdicts were retained.

| Label | Result | Native comparison |
|---|---|---|
| span4444-c4-native-8e00ae2e-20261022a | clean, ref0, turbo to 2032-01-10 | diagnostic: existing 126/126 exact; new 294/350, **56 refusals**, never credited exact |
| span4444-c4-native-b238183b-20261022b | clean, ref0, turbo to 2032-01-11 | new 350/350 and existing 126/126 exact; zero refusals/fallbacks |
| span4444-c4-union-b238183b-20261022c | clean, ref0, turbo to 2032-01-01 | new 350/350 and existing 126/126 exact; zero refusals/fallbacks |

The diagnostic refusals were all the six-row source's plain placement at destination (4,6), across seven
formats, two key states and four operations: 7*2*4=56. The native pointer samples source row0, but the old bridge
source extent rejected destination y6. The corrected runs contain no refused cases, faults, differing pixels,
route mismatches or key-mask errors. No turbo-only difference remained, so a normal-speed rerun was not required.

Both corrected runs compare **168000 full-arena bytes** in the 350 new cases (555 bytes changed by the original),
plus **60480 bytes** in the existing 126 native cases (29651 changed): **228480 bytes per run**, padding included.
Existing native alpha gradients include six SP1 out-of-domain cases, all exact. The direct fixture remains
198/198 exact over 57024 pixels, with zero core rejections/faults; it is separately labeled constructed/direct.
The original is used solely as a comparison oracle or explicitly counted fallback, never as ours' successful
rendered output.

Actual native descriptor modes: formats0..5 unkeyed `FFF`, keyed `0FF`; A8 format6 unkeyed `1FF`, keyed `000`
(keyed A8 is unreachable from this consumer). These mode masks are observations, not claims of all native
keyed-tile combinations. All seven source types 2/4/5/6/7/8/9 have balanced locks/references and Release0.
Destination Shutdown restores lock count0/flags0, clears the renderer, and context/destination Release return0.
Shutdown bridge counters: solid4/4 ours, gradient10/10, alpha6/6, alpha-gradient10/10, blt446/446; fallback0
at every entry. The native fixture deliberately obtains ours' pixels even under `span4444_shadow=1`.

The union request used **exactly the supplied 39 names plus span4444**. The saved request and log audit
(`build-runs/union-audit.json`) confirm all40 enabled, none missing. Both final game runs used DLL SHA prefix
`268a965a`; the union recorded `span4444` as enabled with **0 classes replaced** and installed slots1..5.
Shipped 4444 calls at shutdown are **0 in both runs**, and shipped shadow comparisons are0. Consequently this
is constructed native exactness, not shipped shadow coverage or evidence resolving SP4.

### Questions and next steps

Filed **SP7** in `docs/specs/questions/span-render-4444.md`: native source/destination alias ordering is not
published; overlapping images remain refused pending that contract. SP2 (span bounds) and SP4 (shipped type7
producer) remain open. No default/full-owner replacement credit follows from these fixtures.
The coordinator should regenerate the question index on integration; this clean-room lane edited only the
allowed question file and did not run anything from tools/. No game files or analysis outputs were committed.
The Bash quiet wrapper failed with E_ACCESSDENIED, so capped PowerShell and the required qbuild wrappers were used.
Integrate the supported changes, then seek spec/runtime evidence for SP2/SP4/SP7 before expanding scope.

## Coordinator verification audit (2026-10-22)

Recorded original-versus-Rust same-process comparison sets: span4444_sp6 350/350;
span4444_ctx7 126/126; span4444_direct 198/198. Source is the archived
`span4444-c4-union-b238183b-20261022c` raw log and verdict. Native arenas total228480bytes;
original changes555+29651bytes. All sets have0differences/refusals/fallbacks/faults,
and all6SP1 cases are actually exact (sp1_differing=0); no accepted-difference exception
is counted as exact. Context tests switch actual original vtable entries vs Rust
forwarders over identical input and compare arenas without state resynchronization.
The run reaches2032/ref0, enabled by `enable=span4444`.

This qualifies only the measured same-process renderer comparison. Ordinary gameplay
calls remain0; no shipped-use, replacement/default, full context-owner or untested
alias/failure-path claim is made. SP2/SP4/SP7 remain open.
