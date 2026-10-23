| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzgraphics5: GZ graphics owners | Codex, `core-gzgraphics5` (detached HEAD) | ready: core contracts and 44 vectors; full gate passed; explicit spec gaps retained | Coordinator review/land; settle GZG1–GZG6 before any replacement claim |

## Report

Implemented the widget-facing GZ owners in the sibling
[`gz_graphics`](../../engine/core/src/gz_graphics/README.md) module, separate from
SC4DrawContext. The module README contains the owner closure table by spec
subsection, with code and test items and explicit question IDs for open work.

Context coverage includes independent retained target/engine identity,
fallible attachment, notifications, colour conversion, narrow reset/state
stack, transforms, exclusive-edge clipping and its notification quirks,
primitive input conversion, filled result-code dispatch, source binding and
tiling notification rules, suspension nesting and engine forwards. Buffer
coverage includes format selection, destination lock/reference failure paths,
native allocation/lock quirks, six-word record caching, state and AA grouping
plans. Solid/alpha span writes reuse all five existing software kernels.
Hardware state owns dirty-state and transform ordering; SimGL owns construction,
target probing and the settled specialized pair-count gate. Unsettled device
and AA geometry execution returns Unsupported rather than invented pixels.

Graphics coverage includes identities, lifecycle, dispatch/listener mapping,
borrowed driver getters, identity-sensitive replacement, active-driver mode
availability, downgrade ladder and flags. Alpha exposes all 26 forwarding
operation identities with unchanged delegate arguments, the 32-level count and
null on file-open failure. Canvas owns saved bounds/title, show/focus, packaged
asynchronous posting, code-page fallback, lifecycle, input nesting and message
coordinate updates. Drivers, surfaces and OS windows stay foreign traits. All
state owners are Send + Sync and getters do not use RefCell.

### Validation

- Focused tests: 27 passed, including 21 grouped tests covering all 44
  GZG-V01–V44 vectors and six lifecycle/kernel/thread tests. V24 checks the
  specified AA grouping/result plan; actual AA pixel execution remains GZG2.
- Formatting: PASS, five explicitly owned Rust files, zero diff lines.
- Required full `qbuild.py ... all`: **PASS**, collected to terminal result.
  Cargo: **6302 passed, 0 failed, 0 ignored**; Clippy: **0 diagnostics**;
  native build: **217/217**; full CTest: **68/68**, zero failures.
- Native queue label: `qbuild-20261023-204159-build-ctest`. Full evidence remains
  in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log` and
  `fc4verify-ctest.log`. Native build took 99.7 s; CTest 173.4 s.
- The gate's “tree changed during the run” notice is the journal-only commit
  `0f65cf5b`; source was stable at `6a8d29c0`. No repeated gate was needed.
- The initial focused q.py wrapper could not launch Bash (Win32 error 5),
  preserved in `build-runs/q/20261023-203322-13380.log`; direct Cargo then passed.
- No game run or shadow measurement: core-only task, no plugin/capi wiring.
  C ABI exports and version are unchanged. No generated ABI entries were edited.

### Open sections and questions

Existing spec limits remain explicit; this is not a rendering replacement claim:

- **§§2.3, 3.2–3.3 / GZG1:** gradient conversion, exact geometry/coverage and
  tessellation, unnamed convenience primitives and Blt/BltTiled arguments.
- **§§4.1–4.3 / GZG2:** extension slots, source/device subdivision, buffer AA
  edge pixels, hardware Shutdown details and SimGL specialized/fallback paths.
  Source/gradient bridges remain Unsupported at these declared boundaries.
- **§§5.1–5.3 / GZG3:** unnamed IO signatures, discovery/INI resolution,
  partial-init rollback, screen-format defaults, projection and presentation.
- **§6.1–6.2 / GZG5:** standalone alpha signatures/pixel formulas and successful
  legacy loader quantization/malformed-file handling. Open failure is covered.
- **§§7.1–7.3 / GZG4:** unnamed slots/listener plumbing, native window ownership,
  event-8/9 conditions and Windows event/coordinate translation.
- **§2.1 / newly filed GZG6**, appended to
  `docs/specs/questions/gz-graphics.md`: whether context destruction calls
  engine Shutdown as well as Release. Literal implemented reading: replacement
  shuts down; destruction releases engine then target independently. Tests
  expose this distinction, including the buffer final-release no-unlock quirk.

Next: review/land the core module and its declared boundaries; resolve the
numbered questions before adding pixel, host event or replacement claims.

Commits: `d35b3fbe` initial implementation/vectors; `6a8d29c0` additional
coverage, closure table, formatting and GZG6; `0f65cf5b` intermediate gate journal.
