| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-viewutil29: view utilities, engine | Codex; .claude/worktrees/close-viewutil29; reserved branch close-viewutil29, detached HEAD | blocked: coordinator STOP; code 00c96000e; public ABI unchanged; qbuild PASS, cargo 7898, clippy 0, CTest 124/124; house-r4b clean/ref_warnings=0, shadow totals unparsed; questions VU12/VU17/VU18 | 1. Parse house-r4b. 2. Resolve spec gaps and finish remaining bodies. 3. Run gen27 and full house verification. 4. Enqueue engine only when exact. |

## Handoff
STOP received after close-viewutil29-house-r4b completed; no game request remains outstanding. No merge-queue file written.
This round: ac34c0b05 adds private non-deleting iterator destructor comparisons; 26dd3d4ed attempted MeshPick early-exit fixtures; 00c96000e withdraws those unsafe fixtures and adds VU17 evidence.
Final code compares 42/50 bodies; 8 pending bodies emit calls=complete=0. Whole-owner verified credit is NOT achieved.
Local methods: 005D6900,005D6F10,007F6550,007F65B0,007F6AB0,007F6AF0,007F6B50,007F6CB0,007F6D10,007F6D70,007F6DB0,007F7780,007F8300,0080AE80,0080AEF0,0080B020,0080B090; rotation helper 007F6CC0.
Cached-input methods: 007F6620,007F6690,007F66A0,007F66C0,007F6710,007F6770,007F6BA0,007F7710,007F7750,007F77D0,007F7850,007F78A0,007F78E0,007F7920,007F79B0,007F7A40,007F7E50,007F8390. Camera/frustum inputs are copied; their construction is not checked by these comparisons.
Private-state methods: 007F6DF0 constructor; 007F83F0 missing-renderer update; 007F9080 missing-renderer ray; 007F9B40 warm cache with every optional-output mask; 0080B000 flags bit0 clear; 0080B3B0 indexed/nonindexed iterator advancement.
The destructor compares returned receiver, vtable restoration and all preserved owner/output words across 32 cases; heap deletion is not exercised. Its core test rejects deletion of borrowed storage and checks preserved fields.
Uncompared: 007F6910 draw-box,007F7F60 city-frustum-box,007F9160 viewport-ground,007F91C0 MeshPick,007F9A40 ModelPick,007F9EA0 ground-corners,007F9F50 draw-build,0080B320 iterator-select.
Earlier commits fix camera yaw's PC24 intermediate rounding (aeb7128b9) and constructor padding around byte flags (c7f4a5ee7). No city-width comparison was weakened or compensated.
VU12 remains: city width had 7/1024 mismatches per line; first case width64 native42A73D74/core42A73D75. The spec does not establish the trig store graph. See docs/specs/questions/view-utilities.md.
VU17 remains: mesh/vertex-buffer/group/index resource hierarchy is unpublished. The attempted flat fixture faulted at 007F91DB reading CCCCCCD2; no replacement layout was inferred. VU18 still lacks draw field offsets.
Corrected-source gate: qbuild all PASS; cargo 7898/7898, clippy 0 diagnostics, CMake 51/51 edges, CTest 124/124. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Owned-file fmt passed (2 files).
Obsolete source: house-r4a corrupt, foreign_exceptions=1/ref_warnings=0 from unsupported mesh fixture. Its unchanged evidence: MAIN/build-runs/game-results/close-viewutil29-house-r4a.log. Earlier gate is explicitly obsolete in build-runs/gate-r4a-obsolete.txt.
Corrected DLL SHA256 125B272B1D7700C0D9B95033B682D0AF5C17063FD7D88DB6E49F6FD8EE4E1948.
House profile short check: close-viewutil29-house-r4b clean, ref_warnings=0, end 2026-04-20. Verdict/log: MAIN/build-runs/game-results/close-viewutil29-house-r4b.{verdict,log}. STOP blocked shadow-total and per-method parsing; exactness is not claimed.
gen27-konradshohe-s1-default was NOT submitted this round: STOP rejected the call before request creation. Earlier r2b is old-source evidence only.
Next: parse corrected house log (all 50 method lines, destructor and cumulative counts); resolve VU12/VU17/VU18; supply isolated camera/provider comparisons for the remaining wrappers; run corrected-source gen27 and full house profile, then create an engine queue entry only when all criteria pass.
