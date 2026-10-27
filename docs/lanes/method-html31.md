| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-html31: cHTMLDocument method observations (engine) | Codex; method-html31 worktree; detached HEAD; reserved branch method-html31 | wip; code af22934b4; cargo 8359 passed, clippy 0, CTest 142/142; catalog-r4 and news-r5 clean/ref_warnings=0; four methods exact; HD9 open; ABI unchanged | 1. Calibrate an available advisor/news/help control. 2. Publish HD9 and bind remaining outputs. 3. Coordinator runs house shadows, credits and enqueues. |

Code af22934b4 adds full IsInSet/AppendSet observations and core helpers; fixes the misplaced save-wait brace and premature nested-tick return that made scripted UI dispatch unreachable.
ABI/seam unchanged. DLL SHA256: 6d9be1bbaa65c02b14fd1c49aa09fe9461954c775851e2d0df9a4b7b9a2b7442.
Gate: qbuild all PASS; 8359 Rust tests, clippy diagnostics=0, native build 45/45 edges, CTest 142/142. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Focused: six html_document_shadow tests passed, including return/state mutations, overlapping sets, empty sets and aliasing; owned Rust fmt passed for three files. Bash q.py failed to start; direct Cargo succeeded.
Game evidence root: C:/Users/tlori/FreeCity4/build-runs/game-results/; labels below have .verdict, .log and .reach.tsv.
method-html31-catalog-r4: clean; foreign_exceptions=0, ref_warnings=0, Rust panics=0, HTML mismatches=0, resyncs=0; DLL SHA prefix 6d9be1bb.
catalog-r4: all 18 script actions ran; query and MySim commands plus budget-window request completed. Verdict date 2026-03-14; April snapshot occurred during quit. This is a targeted profile, not the house profile.
catalog-r4 latest monthly snapshot 2026-04-01: calls=56227 complete=9365 partial=46862 mismatches=0 resyncs=0.
Per-method table: [build-runs/html-methods-r4.tsv](../../build-runs/html-methods-r4.tsv), all 73 rows (70 inventory entries and three extra slots), sorted by reach then bytes; calls, complete, mismatches and every missing observation. 36 rows reached.
0x008FEE4A IsInSet: 28 B; calls=9026 complete=9026 mismatches=0 resyncs=0; AL return and complete unchanged set compared.
0x008FF187 AppendSet: 50 B; calls=8 complete=8 mismatches=0 resyncs=0; complete destination union and unchanged source compared.
0x008FEEB8 SetFontSizeTable: 66 B; calls=20 complete=20 mismatches=0 resyncs=0.
0x009006C0 SetSlot15Byte: 10 B; calls=311 complete=311 mismatches=0 resyncs=0.
Four completely compared methods total 154 B (78 B added this round); coordinator owns coverage credit. Other 69 rows remain uncredited, including uninvoked getters/alignment helpers and unobserved lifetime/map helpers.
Largest reached gaps: InitTagList 10635 B and BuildFirstSets 1388 B need calling conventions/global contents; ParseSPAN 939 B, ParseAttributeList 683 B, ParseBODY 608 B and ParseFONTSTYLE 598 B need full state and collaborator traces.
HD9 remains open in docs/specs/questions/html-document.md: character-helper argument ABI/copied-text service blocks CDATA, document whitespace, whitespace stripping and tag splitting. No new question filed.
Other missing observations: complete native attribute-map contents, ordered element/host calls, entity-map/encoding effects, and lifecycle/refcount/download effects; partial comparisons keep complete=0.
method-html31-news-r5: clean/ref_warnings=0; all 11 actions processed, but ticker 0xCA2AEDC0 was SKIPPED: control unavailable (log line 3897). This is not a successful page-driving action.
news-r5 at 2026-03-01: HTML calls=3263 complete=526 mismatches=0 resyncs=0; IsInSet 504/504, AppendSet 8/8, SetFontSizeTable 4/4, SetSlot15Byte 10/10; all method mismatches=0.
House-profile and confirmed advisor/news/help action acceptance remain unmet. Coverage, regress and merge-train scripts are forbidden under tools/; coordinator must run them and flip credit/enqueue.
Next: calibrate an available page control, resolve HD9, bind the remaining outputs, then run the house shadows profile. Both new requests are final; none outstanding. Local merge-queue state is wip, kind engine.
