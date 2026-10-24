| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzgraphics-close12: graphics owner closure | Codex / core-gzgraphics-close12 | ready | Review settled contracts; full native owners remain blocked by GZG1/2/3/5/7 |

## Implementation and owner closure

The settled contracts assigned to the requested owners are implemented in
`engine/core/src/gz_graphics`. Its README maps each spec section to code and
tests or a numbered open contract. This is ready for review and landing; it is
not a claim of full native owner coverage while those questions remain open.
There are no plugin, C ABI, ABI-version or coverage-map changes.

| Owner | Settled contract status | Fully implemented owner? / remaining open sections |
|---|---|---|
| cGZDrawContext | Implemented: attributes/lifetime, transforms, attachment, convenience calls, ordered polygon chains/direct fan, ellipse routes and cubic subdivision/overshoot | No: 3.2 and 3.2.2/4/5 general integer lines and non-AA outline coverage (GZG1); 3.2.2 stroke-quad corner construction/rounding (GZG7) |
| cGZDrawContextRenderTo* | Implemented: interface/state/dispatch, buffer ownership and format paths, five span kernels with gradients, hardware state forwarding and SimGL construction/probing | No: 4.1 extension slots, 4.2 source conversion/AA pixels, 4.3 hardware Shutdown/device subdivision and SimGL specialized/fallback geometry (GZG2) |
| cGZGraphicSystem | Implemented: identities, lifecycle ordering, message/canvas callbacks, driver identity/borrowed getters, stored preference/path setters, downgrade and flags | No: 5.1 unnamed IO inventory, 5.2 driver discovery/INI resolution and failure rollback, 5.3 presentation/projection/screen defaults and folded slots (GZG3) |
| cGZAlphaBlt | Implemented: operation identities/delegation, 32-level count, legacy indexed alpha/mask loaders, ordinary and stretch clipping, failure ownership observations | No: 6.1 standalone operation signatures/pixel kernels/formats/locks (GZG5); 6.3 wrapper clipping is implemented but its final NoClip pixel operation has that same boundary |

Windows procedure translation belongs to the separate Canvas owner in 7.4;
the requested GraphicSystem's 5.2 service callback is implemented. This resume
corrected that distinction in the closure table without changing engine source.

Implementation milestones: `af797c051` adds the geometry, gradient, clipping
and loader bodies; `94d252ffd` adds the closure mapping; `f155a4cd4` corrects
clippy and completes V46 lifetime/stored-driver-setter fixtures. Engine source
has remained stable since `f155a4cd4`.

## Validation

One fresh `qbuild all` was run in this resume and collected through terminal
exit 0 in session 91095. It reported **qbuild: PASS**:

- Workspace: **7112 passed, 0 failed, 0 ignored**, summed across all 45
  `test result:` summaries; includes **45 gz_graphics tests**.
- Clippy with warnings denied: **PASS, 0 diagnostics**.
- Explicit absolute-path fmt check: **8 owned Rust files, diff_lines=0**.
- Queued native request `qbuild-20261024-121844-build-ctest`: **PASS**.
  Build **46/46 edges**, 3.8 s, C/C++ cache 30 hits / 0 misses;
  CTest **81/81**, 0 failures, 143.4 s; total native gate 161.3 s.

Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log`, `fc4verify-ctest.log`; native verdict:
`build-runs/build-results/qbuild-20261024-121844-build-ctest.verdict`.

Conformance fixtures cover the applicable section-8 vectors V01-35, V43-61
and section-8.1 vectors V79-90. Existing canvas V36-42 fixtures remain;
the out-of-scope canvas extension V62-78 is not claimed. V24 verifies the
settled dispatch/grouping plan; unresolved real AA execution returns GZG2.
Tests model foreign ownership/failure observations without unsafe native frees.

Earlier evidence is kept distinct: first all passed 7111 tests but failed
clippy's manual_range_patterns lint (fixed in `f155a4cd4`);
`qbuild-first-test.log`, `qbuild-first-clippy.log` and
`failed-check-evidence.md` preserve that history. Old sessions 16330/56402
were unavailable. The late older native verdict
`qbuild-20261024-113435-build-ctest` passed 260/260 build edges and 81/81
CTest, but noted a changed tree; it was retained and not substituted for the
fresh final gate above.

## Questions, measurements and next steps

Filed **GZG7** in `docs/specs/questions/gz-graphics.md`: cubic stroke-quad
corners, degenerate intervals and intermediate rounding. Existing GZG1,
GZG2, GZG3 and GZG5 delimit the remaining spec-open bodies listed above.
No additional settled body was found missing during the resume audit.

Game runs: **none**. This is a core-only change; no game shadow equality or
replacement claim is made. All numeric results above are build/test results.

Next: coordinator review/landing of the settled implementation, then spec
answers for GZG1/2/3/5/7 before crediting any of these owners as fully closed.
