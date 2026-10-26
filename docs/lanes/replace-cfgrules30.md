| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-cfgrules30 graphics rules; kind engine | Codex / .claude/worktrees/replace-cfgrules30 / detached HEAD; reserved branch replace-cfgrules30; source afc66170e | wip; (a),(b),s1 pass; (c) s0 held on GR11; ABI exports unchanged; Rust 7961, clippy 0, CTest 134/134; shadow 13/13 exact | 1. Resolve GR11. 2. Remeasure s0/house faults. 3. Enqueue engine only after (c) passes. |

- Restored cfgrules28's native owner/registration/tracing dropped from main; both requested merges were already ancestors. Default-off key retained; no shared refs updated.
- Implemented GR10 slot 53/language -1 and slot 58 bases, literal suffixes, independent slot 51 log base; tested worked paths, duplicate selection, required-file failure and no separator repair.
- Fixed SCR11 physical block filtering; tested suppression and per-include state. Added private parse diagnostics, twelve recommendation traces and a snapshot-backed opt-in owner shadow using native name lookup only.
- Corrected source afc66170e, DLL SHA256 prefix f2e48878. Final qbuild all PASS: Rust 7961/0/0, clippy 0 diagnostics, native build 38/38, CTest 134/134 (170.6 s). WT logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log. Owned fmt: 5 files, 0 diffs.
- Game evidence below is in C:/Users/tlori/FreeCity4/build-runs/game-results/; all final corrected-source runs have ref_warnings=0. No requests remain outstanding.
- Query parity PASS: cfgrules30-r1-queries-off2.log / -on2.log: final renderer properties 77/77 exact; recommendations 12/12 exact; both verdicts clean; required rules/card files opened, Init status=0.
- (b) owner shadow PASS with key off: cfgrules30-r1-queries-off2.log, 13/13 SetOptions/Apply snapshots exact, each 77 properties + 12 recommendations; no native property writes forwarded by the shadow.
- (b) cross-setting saves PASS 2/2: queries-off2/on2 saved/exported; cfgrules30-r1-load-off-with-on2.log clean to 2026-06-01 and cfgrules30-r1-load-on-with-off2.log clean to 2026-06-04; matching exported/prepared hashes in verdicts.
- (a) PASS: house default list + cfgrules, cfgrules30-r1-house-on3.log clean through 2032-01-01 (215.578 s); same-DLL off control cfgrules30-r1-house-off3.log also clean through 2032 (363.437 s).
- Earlier corrected-source house2.log:9293 failed at 2026-01-27, null read +0xE2C06; its clean repeat does not explain this intermittent fault (GR11).
- (c) s1-grow PASS: cfgrules30-r1-s1-grow-off1.log clean to 2028-02-01; -on1.log clean to 2028-01-30; no foreign exceptions or reference warnings.
- (c) s0-grow FAIL/unresolved: cfgrules30-r1-s0-grow-off1.log:11409 corrupt at 2026-03-13/+0x3529C0; -on1.log corrupt at 2026-01-30/+0xE2C06; -on2.log:9294 repeats +0xE2C06 at 2026-01-27. These differ from the known quit-time Shutdown fault.
- GR11 filed in docs/specs/questions/graphics-rules.md for the unexplained native rendering/caller-state boundary. No extra restores or native layout assumptions were implemented.
- Superseded source fc88269ee / DLL 2dd28c53: queries-off1/on1 still fell back (20/77 properties and 9/12 recommendations differ); house1's clean 2032 and first cross-load pair do not validate the corrected parser. Diagnostic-only parse-debug1 / DLL 37e2246c isolated physical line 218 before the SCR11 fix.
- cfgrules is NOT ready for the replace list: (c) remains held on GR11 despite the passing gate, exact queries/shadow, (a), (b) and s1. WIP queue entry only; no ready merge-train enqueue or credit promotion.
- Next: resolve GR11 from permitted specs, explain/fix the distinct s0/initial-house faults, repeat the affected controls, then enqueue the published actual tip as kind engine.
