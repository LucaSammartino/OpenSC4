| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-html31: cHTMLDocument method observations (engine) | Codex; method-html31 worktree; detached HEAD; reserved branch method-html31 | ready for verified 154 B; code af22934b4 plus main merge 9c62bbf96; cargo 8385 passed, clippy 0, CTest 143/143; house-r6 clean/ref_warnings=0; four methods exact; HD9 existing/open; ABI/seam unchanged | 1. Coordinator attach house-r6 coverage credit and enqueue the published full tip. 2. Leave the other 69 rows uncredited pending HD9 and complete output bindings. |

Code af22934b4 adds full IsInSet/AppendSet observations and core helpers; fixes the misplaced save-wait brace and premature nested-tick return that made scripted UI dispatch unreachable.
ABI/seam unchanged. Verified DLL SHA256: b6b89d33ce01814f40308003f31c775c2bdb8454f5a852c9cacb9a0ee8d90298.
Gate after main merge: qbuild all PASS, run once; 8385 Rust tests, clippy diagnostics=0, native build 61/61 edges, CTest 143/143. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
Focused tests from implementation remain covered by the full gate; owned Rust fmt PASS, three files (absolute paths; initial relative-path invocation found no files).
Game evidence: C:/Users/tlori/FreeCity4/build-runs/game-results/method-html31-house-r6.{verdict,log,reach.tsv}; request/script retained in this worktree's build-runs/html-house-r6.{req,script}.
House profile: copied the complete [ini] block of main's regress-train288-shadows.req; added HTML compare, reach, action script and close_dialogs; turbo=1, end_date=2026-05-01. No profile comparison was weakened.
house-r6 verdict clean at 2026-05-01; ref_warnings=0, foreign_exceptions=0, Rust panics=0, structured exceptions=0; DLL hash prefix b6b89d33. Queue wait 902 s; game wall 225 s.
Advisor action confirmed: all 15 actions ran (log:4068); mayor command succeeded, advisor tab 0x49EDF9B7 and advisor button 0xAA15C795 posted (log:3686/3701/3731); reach records SetIndividualAdvisorWindow=1 and updateBodyTextCtrl=25.
Latest monthly snapshot 2026-05-01 (log:6683): calls=112911 complete=18251 partial=94660 mismatches=0 resyncs=0.
Per-method table: [build-runs/html-methods-r6.tsv](../../build-runs/html-methods-r6.tsv), all 73 rows, 37 reached; address, bytes, calls, complete, mismatches, reach lower bound and every missing observation.
0x008FEE4A IsInSet: 28 B; calls=17770 complete=17770 mismatches=0 resyncs=0; AL return and complete unchanged set compared (log:6694).
0x008FF187 AppendSet: 50 B; calls=8 complete=8 mismatches=0 resyncs=0; complete destination union and unchanged source compared.
0x008FEEB8 SetFontSizeTable: 66 B; calls=95 complete=95 mismatches=0 resyncs=0 (log:6695).
0x009006C0 SetSlot15Byte: 10 B; calls=378 complete=378 mismatches=0 resyncs=0 (log:6686).
Four completely compared methods total 154 B (78 B from IsInSet/AppendSet); coordinator owns coverage credit. Other 69 rows remain uncredited; partial observations keep complete=0. This is not verification of all 38218 owner bytes.
Largest reached gaps: InitTagList 10635 B and BuildFirstSets 1388 B need calling conventions/global contents; ParseSPAN 939 B, ParseAttributeList 683 B, ParseBODY 608 B and ParseFONTSTYLE 598 B need full state and collaborator traces.
HD9 remains open in docs/specs/questions/html-document.md: character-helper argument ABI/copied-text service blocks CDATA, document whitespace, whitespace stripping and tag splitting. No new question filed.
Other missing observations: complete native attribute-map contents, ordered element/host calls, entity-map/encoding effects, and lifecycle/refcount/download effects; partial comparisons keep complete=0.
Other house diagnostic: propdev::DoMessage reports mismatches=191 complete=0 (log:6646); that owner is outside this HTML verification and receives no claim here.
Earlier news-r5 was clean but skipped its ticker action; it remains distinct from house-r6's successful advisor action. The two qwait timeouts preceded the final house-r6 verdict and are not failed game verdicts.
Final round merged main without new implementation; all source is committed, gate passed, no game request outstanding. Local merge-queue state ready, kind engine, full final HEAD; coordinator attaches credit and invokes tools/merge-train.py under the clean-room separation.
