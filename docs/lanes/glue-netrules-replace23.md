| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-replace23 (network rule replacement hosts) | Codex / `glue-netrules-replace23` (engine lane) | Ready measured partial replacement: cargo 7642/0, clippy clean, CTest 99/99; netrules shadow 73/73; replacement game runs clean through 2032, ref_warnings=0; ABI unchanged | Coordinator review/credit; catalogue command host remains missing |

Code: `50f4fa1bc` adds default-off `enable = netrules`, selecting the Rust manager/tool hosts without inventing standalone solver factories. Explicit host disables veto the group; overlapping keys register each host once. Shadows stay independently opt-in.
Code: `239204aa2` binds city/message context and returns private tools at reference count zero while retaining the cache's creation reference.
Tests: registration, default-off, disable/veto and overlapping keys; all 13 cached tools are distinct, survive manager destruction when retained, and release to zero. Each private type starts at zero; invalid cached/private types return null; unretained created-tool disposal is covered.
Gate: `qbuild all` PASS; cargo 7642 passed/0 failed/0 ignored, clippy 0 diagnostics; queued native verification `qbuild-20261025-064420-build-ctest` build 82/82 edges, CTest 99/99.
Game evidence is under `build-runs/game-results/`; requests/scripts remain under `build-runs/game-queue/done/` and `build-runs/netrules-actions.txt`.
`gnr23-239204-r1-only2032`: netrules alone, clean to 2032-01-15, ref_warnings=0.
`gnr23-239204-r1-full2032`: saved house list plus netrules, clean to 2032-01-01, ref_warnings=0.
`gnr23-r3-allhosts2032`: expanded house list including terrain/effects/disasters/netocc plus netrules, clean to 2032-01-01, ref_warnings=0. Effects retains its existing native-layout registration guard.
Replacement runs load 24 tile files/6295 tile rules and 249 intersection rules; scripted placement solves/inserts 5 cells, failure=0, refused=0. DLL evidence hash prefix: 98d0c9a5; this round changes tests only.
`gnr23-239204-r2-shadow2032 73/73`: netrules exact on every shadow line, mismatches=0, table_mismatches=0, incomplete=0, unavailable=0, resynced=0; clean to 2032-01-05, ref_warnings=0. `gnr23-239204-r1-shadow2032` also 73/73.
Open: original puzzle-piece catalogue command remains unhosted (network-rules section 2.4.3, existing SPEC-GAP comment); intersection observation remains explicitly unobserved. The opt-in host group is partial replacement, not proof of that catalogue. No new questions filed; no coverage or regress tooling edited.
