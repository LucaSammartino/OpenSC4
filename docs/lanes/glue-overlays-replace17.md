| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-replace17 (portable overlay owner factory) | `Codex` / `glue-overlays-replace17` (engine lane, detached HEAD) | Ready measured partial: cargo 7526 passed, clippy 0 diagnostics, ctest 85/85; clean turbo run to 2032, ref_warnings=0; factory 13/13, getters 73/73, records/replay 292/292; ABI unchanged; native replacement incomplete | Supply native class/Init routing and the current house list; complete native replacement and full-house run before adding overlays to that list. |

Code: default-off `overlays` table entry registers FC4-owned CLSID 0xFC401717, private IID 0xFC401718. This is explicitly a synthetic portable owner, not a guessed native CLSID. Native IID 0x895D6544 is rejected.
Core bridge: private static-library transport owns the production terrain_overlays manager behind a Mutex; explicit dimension-bearing inputs support Init/Shutdown, binding decals, removal and alpha access. No public C ABI exports changed.
Tests: three real-DLL CTests cover off/on/disable precedence, factory enumeration, interface/refcount behavior, repeated Init, Shutdown, arena reset and simultaneous getter/setter calls; focused Rust lifecycle test 1/1 passed.
Gate: qbuild all PASS on final source; cargo 7526/0/0, clippy 0 diagnostics; native queue qbuild-20261025-012901-build-ctest, build 55/55 edges, CTest 85/85. Logs: build-runs/qbuild-{test,clippy}.log and build-runs/fc4verify-{build,ctest}.log.
`gor17-bd0738af-owneronly-2032-r3 73/73` — clean, 2032-01-01, ref_warnings=0; DLL SHA256 prefix f62bcdf5; overlays enabled with overlays_shadow and overlays_fixture.
The real game's GZCOM constructed the FC4 owner and its lifecycle check passed 13/13, mismatches=0. Native record observation 292/292 (12264 checks), controlled suffix replay 292/292 (10220 fields), getter fields 7300; all mismatches=0, incomplete=0.
Evidence: build-runs/game-results/gor17-bd0738af-owneronly-2032-r3.{verdict,log}; completed request under build-runs/game-queue/done/.
Open: native manager interception remains 0, explicitly logged. The terrain view still returns native managers; this does not verify native Init/history/drawing or constitute a cSTEOverlayManager replacement. Native routing/resource adapters remain unimplemented and default-off.
The exact full-house run was not performed: its current list was requested asynchronously because tools/regress.ps1 is forbidden to this lane; no reply supplied it. Do not append overlays to the house replace list from this partial work.
Questions: none filed, following revised instructions; SPEC-GAP comments identify unsupported native routing. No specs, tools, profiles, coverage files or run records edited.
