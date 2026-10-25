| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-replace17 (overlay probe safeguards) | `Codex` / `glue-overlays-replace17` (engine lane) | Ready measured partial: cargo 7525 passed, clippy 0 diagnostics, ctest 82/82; two clean turbo runs through 2032, ref_warnings=0; getters 73/73 each, fixture records/replay 292/292 each; ABI unchanged | Specify native construction/Init interception before implementing or crediting the overlays replacement. |

Changes: unavailable requested fixture replay now emits a failing shadow line and contributes 35 mismatches instead of disappearing into a 0/0 replay summary.
Tests: extended observer ctest covers unavailable fixture accounting/reset; new overlays_contract_test checks actual Rust transport, x86 getter slots/output order, untouched placement outputs, borrowed references, default-off behavior and unavailable-load failure.
Gate: qbuild all PASS; workspace 7525 passed/0 failed/0 ignored; clippy zero diagnostics; queued native build 272/272 edges, CTest 82/82 including both overlay tests.
`gor17-750e5ab55-observe-2032 73/73` — clean, 2032-01-19, ref_warnings=0; record observation 292/292 with empty arenas.
`gor17-750e5ab55-fixture-2032 73/73` — clean, 2032-01-01, ref_warnings=0; populated-record observation 292/292 (12264 checks), controlled replay 292/292 (10220 fields), all mismatches=0, incomplete=0.
Evidence: both unique-label `.verdict` and `.log` files under `build-runs/game-results/`; requests under `build-runs/game-queue/done/`; native logs `build-runs/fc4verify-{build,ctest}.log`; DLL SHA256 prefix `1452557b` in both verdicts.
Missing: no overlays replace key/factory was registered. Overlay-manager section 3.8.1 specifies direct construction and direct Init of all four managers, with no class ID or factory interception contract. Header marks this SPEC-GAP; tutorial-style registration cannot intercept those calls.
Replacement-only and full-house replacement runs remain blocked by that construction boundary. These runs verify original-owner probes and a controlled suffix; complete Init/history/image-byte/drawing equivalence remains unmeasured (OV13 and the spec's renderer residuals).
Questions: none filed, following revised run-5 instructions; no tools/profile/coverage files changed. The house replace list must not gain overlays from this partial patch.
