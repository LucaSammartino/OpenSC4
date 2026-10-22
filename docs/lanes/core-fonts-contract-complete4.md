| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts-contract-complete4: cGZFont published contracts | Codex / `core-fonts-contract-complete4` (engine lane) | wip: FN15-FN17 implemented; 64 slots audited; cargo 4248 passed; clippy 0 diagnostics; ctest 67/67; fmt 10 files passed; FN18-FN22 residuals; no ABI change | Resolve FN18-FN22 before cGZFont whole-owner credit; coordinator regenerates question index. |

## Result and scope

Implemented the newly published FN15-FN17 contracts in Rust core only. This is a reviewable implementation milestone, **not cGZFont complete-owner credit**: the residuals below prevent that claim. No font-family coverage promotion, plugin/CAPI work, ABI export/table change, or game files. cGZFontSys retains its separate, already-landed credit; its implementation was not changed.

- `names.rs` owns all 477 character/name mappings from the normative TSV (479 original records, two first-wins duplicates). Binary search and the holder success cache are core-owned; removed the foreign `ps_glyph_name` owner-output hook. Retained aliases are periodcentered/Upsilon; failed retained names go only to uniXXXX, never discarded aliases.
- `advance.rs` owns exact signed-integer × float32-significand arithmetic with extended exponent range, PC24/PC53/PC64, and all four rounding controls. It rounds the product before exact power-of-two scaling and integer truncation. Core owns the near-one non-space bypass, narrowed space adjustment, s16 storage and zero-width overrides. The foreign library supplies only incoming thread control, defaulting to application PC24/nearest-even. Loaded, fixed-size and missing-box paths use this arithmetic.
- Explicit lock/unlock ignore callback failure, wrap the u32 counter, and return true on normal return. Only old-count 0/1 edges call the renderer, passing font identity, target and derived AA mode. Null callback edges panic before counter mutation (portable no-normal-return representation). DrawTextLine checks its automatic lock separately and leaves the count unchanged on failure.
- Audit repairs: borrowed GetRenderer; loaded-glyph insert-unique semantics, including glyph-done error; unsigned negative-height adjustment; failed-Init holder destruction on final release; wrapping AddRef; native zero-line-height fault represented by panic; character-based newline/ellipsis/paragraph trimming, including UTF-16.

## Validation and limits

- Stable final `qbuild.py all` on code head `4be9f373`: cargo workspace 4,248 passed / 0 failed / 0 ignored; clippy exit 0 with 0 diagnostics; CMake 4/4 edges; full ctest 67/67, 140.3 s; fc4verify PASS in 219.4 s. No tree-changed marker.
- Final `qbuild.py fmt` passed all 10 touched Rust files. Its sole diagnostic was an environment path-canonicalization warning, not a formatting diff.
- Earlier quick gate: 2,380 core tests passed and dependents compiled; final all includes the additional UTF-16 test (2,381 core tests).
- Initial baseline run: workspace 4,238 passed, clippy 0 diagnostics, CMake 206/206, ctest 67/67. Tree changed during that initial run, so it is a baseline record rather than stable final evidence. Archived logs are in `build-runs/baseline/`.
- Full final logs remain in `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Initial retries exposed a missing test-provider method and a faulty test assumption that distinct codes have distinct names; both were corrected. An overlapping early build hit DLL locking; subsequent builds ran sequentially.

Ten new tests independently compare all 65,536 UTF-16 codes with the normative 477-row fixture, walk all names through a foreign face with cache/failure/retry checks, and check retained/discarded aliases. Arithmetic checks include published PC vectors, independent f32/f64 boundary sweeps where those are valid exact oracles, signed controls, ties, extended exponents and a PC53-versus-PC64 boundary. Other tests cover fixed and loaded advances, load and done failures, unique cache insertion, missing boxes, renderer references, failed initialization cleanup, lock callback failure/modes/null/wrap behavior, unsigned metrics, faults and UTF-16 layout.

These are spec-data comparisons and constructed foreign-service fixtures, not real-game native equivalence observations. No game request was submitted: this core owner has no plugin/CAPI/shadow seam in this task, so an unchanged game DLL path would not exercise these new font contracts. No shadow-exact, ref-warning, replaced, or whole-owner verified claim is made.

## Whole-owner checklist (all 64 cGZFont slots)

“Reviewed” means traced to the core body and checked against the published portable rule, not separately observed in the native game. Shared helper residuals apply to callers as listed below. `font.rs`, `layout.rs` and `cache.rs` contain the owner bodies; foreign services are limited to font-file/rasterizer, encoding, screen environment and native renderer calls.

| Slot | Method | Audit outcome |
|---|---|---|
| 0 | QueryInterface | FontObj selects supported IDs and AddRefs only on success; unsupported ID leaves count unchanged. |
| 1 | AddRef | Atomic count, wrapping increment/returned count; creator convenience wrapper starts with its acquired reference. |
| 2 | Release | Decrement above one; otherwise shutdown and destructor cleanup, including failed-Init face. |
| 3 | Init | Reviewed existence/probe/open/size/family/default-glyph/renderer sequence and initialised flag. |
| 4 | Shutdown | Reviewed initialized-only renderer detach, caches, palettes and face closure. |
| 5 | GetName | Reviewed string copy. |
| 6 | GetStyle | Reviewed all-mask test. |
| 7 | GetStyles | Reviewed field read. |
| 8 | SetStyle | Reviewed 0x3C mask. |
| 9 | GetFaceKind | Reviewed fixed/scalable state read. |
| 10 | GetSize | Reviewed field read. |
| 11 | SetSize | Reviewed refusal/clamp/face metrics; repaired unsigned adjustment. |
| 12 | GetFontColor | Reviewed field read. |
| 13 | GetFontBackgroundColor | Reviewed field read. |
| 14 | SetFontColor | Reviewed both field stores. |
| 15 | GetAntialiasing | Reviewed field read. |
| 16 | SetAntialiasing | Reviewed zero ignored. |
| 17 | GetGammaCorrection | Reviewed field read. |
| 18 | SetGammaCorrection | Reviewed clamp/change/table rebuild; helper residual FN20. |
| 19 | GetSharpness | Reviewed field read. |
| 20 | SetSharpness | Reviewed clamp/change/table rebuild; helper residual FN20. |
| 21 | GetFilePath | Reviewed string copy. |
| 22 | SetFilePath | Reviewed string assignment. |
| 23 | GetCodePage | Reviewed true and field read. |
| 24 | SetCodePage | Reviewed store/true. |
| 25 | IsCodePageAvailable | Reviewed complete probe table; glyph presence is foreign face service. |
| 26 | IsCharEncodingAvailable | Reviewed encoding == 2. |
| 27 | IsCharAvailable | Reviewed empty/converted probe walk and restoration; incomplete input FN21. |
| 28 | PreloadChars | Reviewed initialized-only converted loading and result; inherited FN18/FN21. |
| 29 | GetScaling | Reviewed pair read. |
| 30 | SetScaling | Reviewed clamps/cache clears/reapply char size. |
| 31 | GetAdvanceScaling | Reviewed (scale, 1). |
| 32 | SetAdvanceScaling | Reviewed clamp/cache clear. |
| 33 | GetSpacingAdjustment | Reviewed pair read. |
| 34 | SetSpacingAdjustment | Reviewed pair store. |
| 35 | GetLineHeight | Reviewed height plus spacing. |
| 36 | CharWidth | Reviewed initialized guard and cached extent. |
| 37 | CharHeight | Reviewed ignored arguments and height read. |
| 38 | GetSizeAboveBaseline | Reviewed ascent plus spacing. |
| 39 | GetSizeBelowBaseline | Reviewed line height minus above baseline. |
| 40 | GetAdvanceWidth | Reviewed uninitialized zero and advance plus spacing; inherited FN18. |
| 41 | CalculateWordsToFitInWidth | Reviewed word/Thai/map-back/unsigned-width rules; invalid progress/backing domain FN22. |
| 42 | CalculateCharsToFitInWidth | Reviewed per-unit advance and source length; incomplete input FN21. |
| 43 | CalculateNumberOfLines | Reviewed wrapping walk; zero-progress adapter remains FN22. |
| 44 | CalculateWidthOfLines | Reviewed unsigned division and 20..4000 binary search; repaired zero-height fault. |
| 45 | CalculateTextArea (box) | Reviewed all branches/default bounds/newline character test. |
| 46 | CalculateTextArea (wrap flags) | Reviewed four rectangle-update branches. |
| 47 | TextArea | Reviewed origin/string width/height rectangle. |
| 48 | GetStringWidth | Reviewed composed first-unit advances and source conversion; FN21/FN22 invalid boundaries. |
| 49 | GetStringVisibleWidth | Reviewed exact space set, returned source length; inherited conversion residuals. |
| 50 | FormatForEllipsis | Reviewed return-original-length, fit/truncate/space/append; repaired encoded-space trimming. |
| 51 | DrawTextLine | Reviewed palette/preload/clipping/pen/renderer/automatic lock; inherited conversion/AA/advance residuals. |
| 52 | DrawTextParagraph | Reviewed placement/alignment/converted CRLF stripping; missing published pen/result contract FN19. |
| 53 | GetRenderer | Repaired borrowed getter: no acquired reference. |
| 54 | SetRenderer | Reviewed same-pointer no-op, new attach before old detach, Arc lifetime ownership. |
| 55 | LockSurfaceForFontUse | FN17 implemented and tested: old-zero callback, ignored return, wrapping increment, true. |
| 56 | UnlockSurfaceForFontUse | FN17 implemented and tested: old-one callback, ignored return, wrapping decrement, true. |
| 57 | deleting destructor | Release models destructor cleanup; Rust Arc owns allocation lifetime, no C++ raw-allocation ABI surface. |
| 58 | GetXOffset | Reviewed cached signed offset. |
| 59 | GetAppropriateCachedBitmapData | Reviewed aaMode 1 mono, all others grey. |
| 60 | GetCharData | Reviewed hit or AddCharData; no false initialized/face guard. |
| 61 | AddCharData | Reviewed index/name/default/load/fixed/offset/advance/box/done error; FN15/FN16 repaired, FN18 residual. |
| 62 | CopyGlyphToSourceData | Reviewed packed-bit unpack, AA mapping, widened cells and underline/strike geometry. |
| 63 | ReallocateSourceData | Reviewed grow-only padded bitmap, row copy and pitch. |

Helper audit also covered constructor defaults, content source probing/load flags, retained PS names/fallback/cache, advance arithmetic, palette split/compose/MRU, AA lookup, encoding probe/conversion/source maps, complete word-class tables, composed/Thai lookahead and presentation-form substitution. File/rasterizer/renderer outputs remain explicitly typed foreign services; none supplies the core name map, computed advance or lock state.

## Questions and residuals

Appended FN18-FN22 to `docs/specs/questions/drawing-fonts.md` (only permitted spec edit). No prohibited tools/index generator was run; the coordinator must regenerate QUESTIONS.md when integrating.

- FN18: non-finite or out-of-i64 x87 conversions and exception masks. Finite representable arithmetic implemented; unsupported conversion returns a typed error, with legacy font adapters panicking rather than inventing a stored value.
- FN19: paragraph pen parameter/writeback and return propagation. Existing no-pen API and provisional result retained with a source comment.
- FN20: AA helper's exact ambient precision/store/pow/log sequence. Existing literal implementation retained, no exact-native precision claim.
- FN21: incomplete UTF-16/multibyte input source-length/output behavior. Existing bounded adapter retained and marked.
- FN22: caller preconditions for native Thai lookahead, mode-2 bounds and zero map-back progress; existing safe adapters remain explicit differences outside the promised valid-input domain.

No further round, restart, branch switch, ref update, merge or push was used. Commits are detached as requested; supervisor publishes them. Git printed a denied packed-refs.lock attempt during the initial three commits, but the detached-HEAD commits succeeded; no manual shared-ref/config operation was attempted.

## Commits and next steps

- b688951c: FN15-FN17 core implementation and independent tests (substantive milestone within the first 30 calls).
- f5ea95e9: ownership, cache uniqueness, failure/metric audit repairs and explicit residual questions.
- 4be9f373: character-based layout fixes and UTF-16 tests; final gate code head.
- Resolve FN18-FN22 from primary evidence, then implement and verify those exact contracts before any cGZFont complete-owner credit. Review this milestone independently of cGZFontSys's existing credit.
