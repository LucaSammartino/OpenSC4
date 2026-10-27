| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-html31: cHTMLDocument method observations (engine) | Codex; method-html31 worktree; detached HEAD; reserved branch method-html31 | continue; code e62de3543; cargo 8358 passed, clippy 0, CTest 142/142; r3 clean/ref_warnings=0; two methods exact; HD9 open; ABI unchanged | 1. Finish the advisor/help action with turbo off during UI steps. 2. Resolve HD9 and bind remaining outputs. 3. Run house shadows and have the coordinator credit/enqueue. |

Code commits: b06f7d9d5, b27bb0127, bbe25c3fc, e62de3543. No public C ABI or seam change.
Final gate on e62de3543: qbuild all PASS; 8358 Rust tests, zero clippy diagnostics, CTest 142/142. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Owned Rust fmt: 2 files passed. Initial quick had 691 passing tests but a stale/failed native build; unpublished static-helper conventions were removed before the successful full gates.
method-html31-catalog-r1: rejected before launch (script belonged under [ini]); not game evidence.
method-html31-catalog-r2 on bbe25c3fc: corrupt, foreign AV 009106d9 through the unsupported HTMLAdvance temporary adapter; ref_warnings=0. Superseded by removal of that adapter in e62de3543.
method-html31-catalog-r3 on e62de3543: clean, foreign_exceptions=0, ref_warnings=0, Rust panics=0, resyncs=0; DLL SHA prefix 086d3f26.
Run evidence: C:/Users/tlori/FreeCity4/build-runs/game-results/method-html31-catalog-r3.{verdict,log,reach.tsv}.
Latest monthly snapshot 2049-05-01: 3,870,951 entries, 26,358 complete, 0 mismatches; all other entries explicitly incomplete.
Method 0x008FEEB8 SetFontSizeTable: 66 bytes; calls=319 complete=319 mismatches=0 resyncs=0.
Method 0x009006C0 SetSlot15Byte: 10 bytes; calls=26039 complete=26039 mismatches=0 resyncs=0.
Potential method credit: 76 bytes only. GetFontSizeTable, SourceText and both alignment bindings had no observed invocation; no credit claimed.
Per-method table: [build-runs/html-methods-r3.tsv](../../build-runs/html-methods-r3.tsv), 73 rows including the 70 named-inventory entries and three extra slots; 36 rows reached. Sorted by reach then bytes; includes calls, complete, mismatches and missing observations.
Largest reached gaps: InitTagList 10635 B and BuildFirstSets 1388 B (one reach entry each; calling conventions and full global contents unbound), ParseSPAN 939 B, ParseAttributeList 683 B, ParseBODY 608 B, ParseFONTSTYLE 598 B.
HD9 in docs/specs/questions/html-document.md blocks ParseCDATA, SkipDocumentWhitespace, StripWhiteSpace and SplitTag: character-helper argument ABI/safe copied-text service missing; complete remains 0 for every input.
Other parser methods have entry census only: complete owner post-state, attribute-map contents and ordered element/host calls still need bindings. Entity-map/encoding and lifecycle/refcount observations also remain absent.
Entry shells refused 008fe7b8, 009065c5, 00908122; reach tracked these instead. No unsupported prologue or convention was guessed.
House/action acceptance NOT met: script stopped before the first tick-driven uicommand completed; requested 2026-03-03 end was overshot to 2049-05-03. Clean verdict is not an action/profile pass.
Next: start the UI script with turbo off, drive an advisor/news/help page, then enable turbo after UI actions finish; use the shared main queue and new labels. All three existing requests have final verdicts.
Coverage/map and merge-train tools were not run because tools/ is forbidden to this clean-room lane. Coordinator owns credit and final enqueue.
Continuation checkpoint: full gate passes, no outstanding game request, no merge-queue entry; restart to finish remaining method observations and house-profile acceptance.
