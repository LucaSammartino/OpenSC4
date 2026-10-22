| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span1555-context4 (1555 native context) | `glue-span1555-context4` / (engine lane, detached HEAD) | ready: cargo 4175/0, clippy clean, ctest 67/67; native and literal-list union clean through 2032/ref0; native 980/980 exact per run, 470400 bytes; shipped 0; ABI unchanged | Keep opt-in; SR1555-2/SR1555-6 remain open; no full-owner/default credit. Round 2 reserved if needed. |

## Report

Round 1 complete; round 2 of the logical family's two remaining rounds is reserved for follow-up.
Worktree reused at its reserved detached HEAD. No branch/ref changes, merges, tools edits, or C ABI changes.

### Contracts and implementation

- 1555 §§1.3 and 6.1 explicitly share the framework 2D context with 4444 §1.3/SP6. This establishes applicability of native setters, destination placement, reciprocal scales, source tile edges, and texture/destination lock lifetimes. No 3D context contract was transferred.
- SP1's 4444 packed arithmetic is renderer-specific. Existing 1555 §5 math already conforms: weight is the wrapped `0x84` accumulator shifted by 26, with five-bit fields and 32-bit packed R/B intermediates. Core math remains unchanged. Added independent weight17..31 gradient vectors and fixed carry/borrow/mask vectors above32.
- `958c633b` landed supported code before call30: exact site+3 caller classification for 1555 with compile-time acceptance/rejection checks at all seven sites. Removed the 4444-only SP1 exception from 1555 comparison accounting.
- `4e4b52e9`, `4aa655a3`: renderer-specific native source cases using shared FC4SpanContext helpers. Exercise all seven source formats, keyed/unkeyed variants, all12 blit kinds, six published nonorigin/scale/rounding cases, nonorigin tiles, null-source no-write, output rectangles and stored context fields. Compare entire destination arenas including padding. A call that never reaches a renderer receives no output credit.
- `4034791b`: source lifetime probes use balanced audited public AddRef/Release; no assumed buffer refcount storage. Destination/context offsets used elsewhere are explicitly published. Shutdown precedes final Release, and matching lock/unlock flags are checked.
- New 1555 format/key/kind counters validate actual native records and retain row13 as unreachable. Renderer-specific helpers are separate files; shared Case defaults preserve existing sibling routes and the sibling classifier/SP1 accounting.

### Verification

- Initial `qbuild all`: PASS, cargo4173/0, clippy0 diagnostics, ctest67/67.
- Final `qbuild quick`: PASS, core2318/0, dependent crates compiled, CMake4/4 edges.
- `qbuild fmt` on the touched Rust test file: PASS, exit0. Absolute file path required by wrapper.
- Final `qbuild all`: PASS, cargo4175/0, clippy0 diagnostics, CMake2/2 edges, ctest67/67 in96.9s. Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. The tree-changed stamp reflects documentation commits; no engine code changed after the public reference-probe fix.

### Real game evidence

All labels were unique; each was waited through `build-runs/wait-span1555-verdict.py`, without qwait. All completed verdicts remain valid and retained.

| Label | Verdict/date | Native comparison | Setter/lifetime checks | Shipped |
|---|---|---|---|---|
| span1555-c4r1-native-20261022-01 | clean/ref0, 2032-01-11 | 980/980 exact, 470400 bytes | Initial fixture error: 312/371 failures per destination due to unsupported raw refcount offset; corrected in4034791b | 0 |
| span1555-c4r1-native-20261022-02 | clean/ref0, 2032-01-03 | 980/980 exact, 470400 bytes | 742/742 passed | 0 |
| span1555-c4r1-union-20261022-01 | clean/ref0, 2032-01-01 | 980/980 exact, 470400 bytes | 742/742 passed | 0 |

For each passing native/union run:

- Type4 and type6 destinations each compare490 cases across235200 arena bytes, including two padding bytes per row. Original drawing changes35516 and35527 bytes respectively (71043 total); these are real writes, not empty comparisons.
- Native routes total980: solid8, gradient20, alpha12, alpha-gradient20, blit920. Direct constructed fixture separately158/158 exact. Other caller count0; shipped count0 at every site through2032.
- All13 reachable format/key rows exercise all12 blit kinds. Formats0..5 unkeyed rows each88 native calls, keyed rows each56; format6 unkeyed56, keyed0. Invalid texture records0. The texture counter's `other` includes the separately classified direct fixture; it is not shipped evidence.
- Inspected every refusal/fallback category: direct ours_refused0/skipped0/diff0; native ours_refused0/faults0/route_mismatch0/no_renderer_call0/diff0; texture lock counts moved0. No original fallback was needed. There was no turbo-only byte difference requiring a normal-speed rerun.
- Both destination lock counts/flags restore0/0, both contexts and destinations Release->0; every source lifetime and texture release is balanced. Runtime ref_warnings0.

The union request uses exactly the supplied keys plus span1555:
`landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones, development, tract, history, clock24, commercial, industrial, civic, parks, lotdev, zonedev, traffic, automata, netmgr, netlot, nettool, demolition, occmgr, propmgr, lotcfg, citysit, propdev, simgrids, citydetail, span1555`.

### Questions and scope

No new spec questions. SR1555-2 and SR1555-6 remain open; zero shipped calls does not answer them. The opt-in complete renderer bridge still compares independent core output against original output; fixtures supply no full context-owner replacement or default-list credit. No sibling behavior or native context owner is replaced by this lane. QueryInterface-rejection and texture-lock-failure injection were not exercised; no coverage is claimed for them.

Next: merge as an opt-in 1555 context validation improvement; retain the existing shipped-use questions and reserve round2 only if follow-up evidence warrants it. Coordinator regenerates shared tables; lane did not execute forbidden tools scripts.

## Coordinator verification audit (2026-10-22)

Recorded original-versus-Rust same-process comparison sets: span1555_ctx4 490/490;
span1555_ctx6 490/490; span1555_direct 158/158. Source is the archived
`span1555-c4r1-union-20261022-01` raw log and verdict. Native arenas total470400bytes,
71043 changed by original; zero differences, refusal/fallback, faults, route/no-call
errors or resynchronization. Runtime reaches2032/ref0. The comparison saves original
output, restores identical input, invokes independent Rust and compares whole arenas;
this is not comparing Rust to itself. Enabled by `enable=span1555`.

This qualifies only the measured same-process renderer comparison. Ordinary gameplay
renderer calls remain0; no shipped-use, replacement/default, full draw-context owner,
or unexercised failure-path claim is made. SR1555-2/SR1555-6 remain open.
