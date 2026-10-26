| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzf29 / gz-framework-core GZF6 + GZF12; kind docs | Codex / re-gzf29 / reserved re-gzf29 branch (detached HEAD) | ready; commits bfaacf9eb, 450207aa5, 75ea6e700; ABI exports unchanged, no ABI-row proposals; docs checks passed; cargo/clippy/ctest n/a; game/shadow n/a; GZF6/GZF12 answered | 1. Land docs queue. 2. replace-gzfw29 implements and qualifies against the recovered contracts. |

- Merged replace-gzfw29 ancestry with only its question-file changes; preserved the current implementation tree without reading engine files.
- Completed docs/specs/gz-framework-core.md §6: all 47 primary buffer entries, signatures and return channels; storage ownership, disposal/reinitialization and null-allocation success.
- Corrected automatic types 1/3: driver format resolution, no palette or foreign-storage attachment operation; GetSize can overestimate allocated storage.
- Specified packed color/key behavior, native alpha-unmap quirks, fill/line/copy conversion and overlap, with instruction-address evidence.
- GZF12: borrowed cache getter; primary IID 0xFA59F7B9; old-release / Init / retain order; false-Init and null-input behavior; cache lifetime and modification-counter invalidation.
- Added §6.5 hardware lifecycle and §6.6 GZB1–GZB8 worked cases covering creation, fill, caching, disposal/reinitialization, conversion, failure and release.
- Moved both original entries to docs/specs/questions/answered/gz-framework-core.md with dated answers; regenerated the question index.
- Validation: questions.py check passed; diff --check passed against the round base; executable audit covered 47/47 slot numbers and matched 43/43 explicit target entries; worked numeric assertions passed.
- Gate: docs-only exemption; no cargo, clippy, ctest, engine conformance edits, game runs or replacement-credit claim.
- Evidence used the local export, decompilation and executable table words; numeric rules checked against instructions. No REA calls or fallback errors.
- Temporary evidence/check helpers remain untracked under build-runs/; no game files, decompiler output or external files committed.
- No remaining uncertainty is parked under GZF6/GZF12; other framework questions remain outside this task.
