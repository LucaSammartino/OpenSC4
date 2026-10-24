# Text editor: attributed-body checklist

Companion to [text-edit.md](text-edit.md). This is the exact **direct-label**
inventory from the Windows export queried on 2026-10-22: 134 unique entries,
19,415 body bytes. The [round 2 attribution audit](text-edit-attribution.md)
reconciles actual coverage's 150 bodies / 19,999 bytes, including all direct
entries and sixteen measured additions. P = a substantive contract is present in the cited section,
B = only a bounded call-site contract is present, U = method-specific
behavior remains unspecified. P does not close foreign primitive contracts.
Addresses and byte counts are metadata, not decompiler listings.

## Direct-label inventory

| Address | Bytes | Export method label | Contract disposition |
|---|---:|---|---|
| 0x0091880E | 26 | GetTextLength | P; text-edit-adapters.md, output-string call (label is not signature) |
| 0x0099357F | 7 | GetTextGutterX | P; text-edit-adapters.md, raw T+0x80 |
| 0x00993586 | 7 | GetTextGutterY | P; text-edit-adapters.md, raw T+0x84 |
| 0x00994A39 | 7 | GetText | P; text-edit-adapters.md, numeric byte length |
| 0x00994E6A | 7 | GetMaximumTextToAllow | P; text-edit-adapters.md, raw maximum |
| 0x00994E7E | 7 | GetFirstVisibleLine | P; text-edit-adapters.md, raw first visible |
| 0x00994E85 | 7 | GetLineHeight | P; text-edit-adapters.md, raw line height |
| 0x00994E8C | 45 | SetText | P; sections 2-5 |
| 0x00994EB9 | 7 | GetCaption | P; text-edit-adapters.md, embedded original-string address |
| 0x00994EC0 | 23 | SetCaption | P; text-edit-adapters.md, string setter/notify=false |
| 0x00994ED7 | 13 | SetMaxUndoLevelCount | P; section 6, including no immediate pruning |
| 0x00994FF2 | 10 | GetLineCount | P; text-edit-adapters.md, vector count/no minimum |
| 0x009BC5E1 | 45 | QueryInterface | P; section 1 (constructor sequence bounded) |
| 0x009BC60E | 39 | ToggleCaretVisibility | P; sections 2-5 |
| 0x009BC635 | 39 | MakeCaretVisible | P; sections 2-5 |
| 0x009BC65C | 89 | StartCaretTimer | P; sections 2-5 |
| 0x009BC6B5 | 35 | GZOnKeyUp | P; text-edit-adapters.md, Escape message/return |
| 0x009BC6D8 | 45 | GZOnMouseDownR | P/B; section 10; TE7 |
| 0x009BC705 | 88 | GZOnMouseWheel | P/B; section 10; TE7 |
| 0x009BC75D | 64 | GZOnSetFocus | P/B; section 10; TE7 |
| 0x009BC79D | 64 | GZOnKillFocus | P/B; section 10; TE7 |
| 0x009BC7DD | 63 | SetFlag | P/B; section 10; TE7 |
| 0x009BC81C | 27 | SetFlag | P/B; section 10; TE7 |
| 0x009BC837 | 29 | ClearFlag | P; text-edit-adapters.md, conditional full-flags dispatch |
| 0x009BC854 | 16 | IsFlagSet | P; text-edit-adapters.md, any mask bit |
| 0x009BC864 | 42 | GetScrollBarImageVertical | P; text-edit-adapters.md, borrowed image/rectangle/write order |
| 0x009BC88E | 42 | GetScrollBarImageHorizontal | P; text-edit-adapters.md, borrowed image/rectangle/write order |
| 0x009BC8DB | 41 | SetFillColor | P; text-edit-adapters.md, base result/bit 0x20 invalidate |
| 0x009BC904 | 74 | SetFontStyle | P/B; section 10; TE7 |
| 0x009BC94E | 25 | GetFontStyle | P; text-edit-adapters.md, nullable borrowed output |
| 0x009BC967 | 20 | SetColor | P; section 10, unchecked color index |
| 0x009BC97B | 22 | GetColor | P; section 10, unchecked color index/output |
| 0x009BC991 | 27 | GetBackgroundImage | P; text-edit-adapters.md, borrowed pointer/option byte |
| 0x009BC9AC | 93 | SetBackgroundImage | P; section 10, identity/ref order and teardown asymmetry |
| 0x009BCA09 | 48 | SetTextGutterY | P/B; section 10; TE7 |
| 0x009BCA39 | 50 | InsertText | P/B; sections 6-7; TE3/TE4 |
| 0x009BCA6B | 50 | RemoveText | P/B; sections 6-7; TE3/TE4 |
| 0x009BCA9D | 97 | RemoveText | P/B; sections 6-7; TE3/TE4 |
| 0x009BCAFE | 22 | SetInsertMode | P; sections 2-5 |
| 0x009BCB14 | 12 | GetInsertMode | P; text-edit-adapters.md, inverse overwrite bit |
| 0x009BCB2E | 23 | GetInsertionLineAndColumn | P; text-edit-adapters.md, ordered unchecked output writes |
| 0x009BCB45 | 88 | SetCaretPeriod | P; text-edit-adapters.md, unsigned bounds/flag reapply |
| 0x009BCB9D | 17 | GetCaretPeriod | P; text-edit-adapters.md, unchecked output |
| 0x009BCBAE | 23 | GetSelection | P; text-edit-adapters.md, stored endpoint write order |
| 0x009BCBC5 | 73 | CalculateVisibleWidth | P/B; section 8; TE5 |
| 0x009BCC0E | 73 | CalculateVisibleHeight | P/B; section 8; TE5 |
| 0x009BCC57 | 29 | SetSoundID | P; text-edit-adapters.md, signed upper-only index test |
| 0x009BCC74 | 31 | StartScrollTimer | P; section 10, running-state gate and interval |
| 0x009BCC93 | 28 | StopScrollTimer | P; section 10, running-state gate |
| 0x009BCCAF | 31 | ScrollTimerCallback | P; section 10, nullable dispatcher boundary |
| 0x009BCCCE | 236 | DoCursorScroll | P; text-edit-layout.md, thresholds/priority/routing |
| 0x009BCDBA | 131 | RemoveScrollBar | P; text-edit-adapters.md, child/remove/release/clear ordering |
| 0x009BCE3D | 454 | SetScrollBarSizes | P; text-edit-layout.md, four bar-presence cases |
| 0x009BD003 | 172 | Shutdown | P/B; section 10; TE7 |
| 0x009BD0E1 | 139 | DoMessage | P/B; section 10; TE7 |
| 0x009BD16C | 47 | CaretTimerCallback | P; section 10, nullable dispatcher and temporary cleanup |
| 0x009BD19B | 50 | GetTextArea | P/B; section 8; TE5 |
| 0x009BD1CD | 41 | GZOnMouseUpL | P/B; section 10; TE7 |
| 0x009BD1F6 | 130 | GZOnMouseMove | P/B; section 10; TE7 |
| 0x009BD278 | 129 | SetScrollBarImageVertical | P; section 10, reference/rectangle/callback order |
| 0x009BD2F9 | 129 | SetScrollBarImageHorizontal | P; section 10, reference/rectangle/callback order |
| 0x009BD37A | 31 | SetCaretColor | P; text-edit-adapters.md, +0x200 conversion/store/no invalidate |
| 0x009BD399 | 32 | GetCaretColor | P; text-edit-adapters.md, +0x1FC/output/true |
| 0x009BD3B9 | 7 | GetFont | P; text-edit-adapters.md, borrowed raw font |
| 0x009BD3C0 | 33 | SetFontColor | P; section 10, converted default color |
| 0x009BD3E1 | 37 | SetFontColor | P; section 10, indexed conversion/dispatch |
| 0x009BD406 | 65 | GetFontColor | P; section 10, success-only output write |
| 0x009BD447 | 56 | ClearFontColor | P; section 10, sentinels and unchecked index |
| 0x009BD47F | 22 | GetHighlightFillColor | P; text-edit-adapters.md, direct +0x1FC result |
| 0x009BD495 | 31 | SetHighlightFillColor | P; text-edit-adapters.md, measured +0x1FC/store/no invalidate |
| 0x009BD4B4 | 48 | SetOutlineUse | P; section 10, bit/color and no invalidation |
| 0x009BD4E4 | 44 | GetOutlineUse | P; section 10, bit and reverse-converted output |
| 0x009BD510 | 68 | GetIndexFromPosition | P; text-edit-adapters.md, mapped line-start sum/coherent-array domain |
| 0x009BD554 | 63 | GetCharacterCountOfLine | P; text-edit-adapters.md, byte span including LF |
| 0x009BD593 | 125 | GetLineAndColumnFromIndex | P; text-edit-layout.md, valid-index domain/LF branch |
| 0x009BD610 | 94 | GetIndexFromLineAndColumn | P; text-edit-layout.md, whole-text clamp/status |
| 0x009BD66E | 70 | SetInsertionIndex | P; text-edit-adapters.md, clamp/recompute/invalidate |
| 0x009BD6B4 | 102 | SetInsertionLineAndColumn | P; text-edit-adapters.md, independent line/column clamps |
| 0x009BD71A | 224 | IncrementInsertionPositionByComposedCharacter | P; text-edit-layout.md, signed selector predicate/helper domain |
| 0x009BD7FA | 163 | IncrementInsertionPosition | P; text-edit-layout.md, 8/100 stepping and final clamp |
| 0x009BD89D | 250 | IncrementInsertionPositionByWord | P; text-edit-layout.md, byte whitespace transitions |
| 0x009BD997 | 91 | SetSelection | P; sections 2-5 |
| 0x009BDA65 | 155 | FixInsertionPositionIfBroken | P; text-edit-adapters.md, empty reset/active-only clamp |
| 0x009BDB00 | 131 | GetPixelWidthOfLongestLine | P; text-edit-layout.md, original-text ranges and unsigned maximum |
| 0x009BDB83 | 334 | GetHorizontalLineMetrics | P; text-edit-layout.md, exact boundary/width loops |
| 0x009BDCD1 | 241 | SetScrollBarValues | P; text-edit-layout.md, ranges/division/callback order |
| 0x009BDDC2 | 48 | GetHorizontalScrollPixels | P; text-edit-layout.md, original-prefix versus pixel offset |
| 0x009BDE57 | 48 | ScrollLineUp | P; text-edit-adapters.md, signed positive first-visible gate |
| 0x009BDE87 | 66 | ScrollLineDown | P; text-edit-adapters.md, signed remaining-row gate |
| 0x009BDEC9 | 112 | ScrollLeft | P; text-edit-layout.md, character/pixel modes and no zero clamp |
| 0x009BDF39 | 142 | ScrollRight | P; text-edit-layout.md, visible-width cap |
| 0x009BDFC7 | 66 | ScrollToStart | P; text-edit-adapters.md, byte/vertical gate ignores pixel-only offset |
| 0x009BE009 | 90 | ScrollToEnd | P; text-edit-adapters.md, signed gate and horizontal reset |
| 0x009BE063 | 70 | ScrollPageUp | P; text-edit-layout.md, native status/update domain |
| 0x009BE0A9 | 104 | ScrollPageDown | P; text-edit-layout.md, signed gate/uncapped increment |
| 0x009BE111 | 104 | ResizeWindowForExactLineHeights | P; text-edit-adapters.md, unsigned division/minimum-one/zero-height failure |
| 0x009BE179 | 192 | GetColumnFromPosition | P/B; section 8; TE5 |
| 0x009BE239 | 162 | GetPositionFromLineAndColumn | P/B; section 8; TE5 |
| 0x009BE2DB | 39 | GetText | P; text-edit-adapters.md, bounded copy/count/no NUL |
| 0x009BE302 | 373 | PutInsertionPositionInView | P; text-edit-layout.md, signed three-quarter stepping/dirty gate |
| 0x009BE477 | 413 | AddScrollBar | P; text-edit-layout.md, attempt gates/resource/failure/call order |
| 0x009BE614 | 186 | FixBadCharacterComposition | P/B; text-edit-adapters.md, owner erase/previous state; foreign UTF16/Thai (TE5) |
| 0x009BE6CE | 225 | SelectCurrentWord | P; text-edit-layout.md, byte scans/existing endpoints/unsafe-end domain |
| 0x009BE7AF | 179 | TestForPossibleScrollBarAdditionOrRemoval | P/B; section 8; TE5 |
| 0x009BE862 | 227 | GZOnMouseDownL | P/B; section 10; TE7 |
| 0x009BE945 | 37 | UpdateScrollBarsAndWindowPosition | P; text-edit-adapters.md, initialization gate/unconditional invalidate |
| 0x009BE96A | 53 | FindText | P/B; text-edit-adapters.md, primitive routing/result filter; foreign search (TE8) |
| 0x009BE99F | 16 | ShowWindow | P; text-edit-adapters.md, update before base show/hide |
| 0x009BEA28 | 1850 | GZPaint | P/B; section 9; TE6 |
| 0x009BF162 | 387 | GetLineAndColumnFromPosition | P; section 9, numeric/pixel/password branches |
| 0x009BF2E5 | 80 | AdjustLinePositionArray | P; text-edit-adapters.md, pre-resize indexed read/unsafe grow domains |
| 0x009BF335 | 171 | RemoveBadCharacters | P/B; text-edit-adapters.md, LF chunk filtering/copy primitive boundary |
| 0x009BF3E0 | 312 | CalculateAllLineLengths | P/B; section 8; TE5 |
| 0x009BF578 | 63 | RemoveBadCharacters | P; text-edit-adapters.md, buffer delegation/resize |
| 0x009BF5B7 | 447 | InsertText | P; section 6, lowered-limit selector/failure domains |
| 0x009BF776 | 306 | RemoveChar | P; section 6, empty/invalid-position boundary |
| 0x009BF8A8 | 227 | RemoveText | P/B; sections 6-7; TE3/TE4 |
| 0x009BF98B | 233 | RecalculateAll | P/B; section 8; TE5 |
| 0x009BFB9C | 265 | Init | P/B; section 10; TE7 |
| 0x009BFCA5 | 74 | SetArea | P; text-edit-adapters.md, unconditional base/layout/bar/invalidation order |
| 0x009BFCEF | 215 | SetFlags | P/B; section 10; TE7 |
| 0x009BFDC6 | 61 | SetFont | P/B; section 10; TE7 |
| 0x009BFE03 | 72 | SetTextGutterX | P/B; section 10; TE7 |
| 0x009BFF52 | 28 | ~cGZWinTextEdit | P/B; section 10; TE7 |
| 0x009BFF6E | 471 | InitializeMemberVariables | P; section 1 (constructor sequence bounded) |
| 0x009C01AF | 207 | cGZWinTextEdit | P; section 1 (constructor sequence bounded) |
| 0x009C027E | 280 | Undo | P/B; sections 6-7; TE3/TE4 |
| 0x009C0396 | 280 | Redo | P/B; sections 6-7; TE3/TE4 |
| 0x009C04FE | 399 | SetText | P; sections 2-5 |
| 0x009C068D | 210 | AddUndoRecordForTextInsertion | P; section 6, native quarter eviction/range helpers |
| 0x009C075F | 130 | AddUndoRecordForTextTextDeletion | P; section 6, absent capacity check |
| 0x009C07E1 | 437 | InsertTextHighLevel | P; sections 2-5 |
| 0x009C0996 | 255 | GZOnCharacter | P; sections 2-5 |
| 0x009C0A95 | 2938 | GZOnKeyDown | P/B; section 7; TE5 |

## Attribution exclusions and additional native bodies

The full name search yields 146 entries / 19,908 bytes. Beyond the 134 above,
it includes five 11-byte adjusted thunks (0x00994F37, 0x00994F6E, 0x009BFB86,
0x009BFB91, 0x009BFF47), folded maximum setter 0x00994E71 (13 bytes),
foreign factory 0x009989D0 (31 bytes), nested UndoRedoAction constructor
0x009BE9CE (31 bytes), and four STL/allocator bodies 0x009BF530 (72),
0x009BFE4B (184), 0x009C0145 (56), 0x009C04CB (51). None independently
proves additional text-editor behavior or family coverage.

Important native bodies absent from direct-label inventory:

| Address | Bytes | Evidenced role / remaining boundary |
|---|---:|---|
| 0x009BC5CC | 21 | Space/LF predicate; section 3 |
| 0x009BD0AF | 50 | Preceding-LF insertion correction; section 7 |
| 0x009BD9F2 | 115 | Sound adapter mislabelled as folded font setter; section 10 |
| 0x009BDDF2 | 76 | Unresolved helper; TE8 |
| 0x009BDE3E | 25 | Action allocation request: zero count returns null, otherwise count*60 bytes; overflow/allocator semantics remain foreign |
| 0x009BE9AF | 31 | Action destruction: owned string at action+0x14, then GlobalHeapFree only when deleting flag bit 1; section 6 |
| 0x009BE9ED | 32 | Creates initial line-start entry; call-site contract section 8 |
| 0x009BEA0D | 27 | Nondeleting action destruction over a valid half-open 60-byte-stride range; section 6 |
| 0x009BF518 | 24 | Range-destruction forwarding wrapper; section 6 |
| 0x009BFA74 | 30 | Destroys every action then frees nonnull vector storage; sections 6/10 |
| 0x009BFA92 | 41 | Range copy-construction in 60-byte strides; destination-null check per iteration; allocator construction is foreign |
| 0x009BFABB | 40 | Counted copy-construction in 60-byte strides; destination-null check per iteration; allocator construction is foreign |
| 0x009BFAE3 | 163 | Nondeleting destructor; section 10 |
| 0x009BFF03 | 68 | Assigns scalar fields, owned string and five-word state to a valid existing action; section 6 |
| 0x009C017D | 50 | Forward action assignment over a valid 60-byte-stride range; section 6 |
| 0x009C04AE | 29 | Range-assignment forwarding wrapper used by the mislabelled vector erase; section 6 |

Other folded/unnamed text-vtable entries include 0x009BC8B8, 0x00994F79,
0x009C32AC, 0x009BCB20, 0x009D41A1 and 0x00994EE4. A foreign-looking
folded name does not establish the text slot's meaning. GetTextLength at
0x0091880E actually copies bytes to an output string, whereas GetText at
0x00994A39 returns end-minus-begin; do not derive signatures from labels.
The text vtable has 90 listed entries; the primary window vtable has 156.
Inherited base/stub slots are not additional primary text-editor bodies.

## Whole-owner acceptance checklist

Final round 3/3 direct-body reconciliation: **97 P / 10,136B**, **37 P/B /
9,279B**, **0 U**; together 134 / 19,415B. A P/B row contains a substantive
owner or call-site contract with bounded dependencies; it is not counted as
a complete body/owner proof. Historical TE3/TE4/TE6/TE7 references in rows
name the questions that motivated their contracts; those questions are
answered, not newly reopened. The remaining exact scope is:

| Direct P/B body group (addresses identify every row) | Bounded dependency / unclosed scope |
|---|---|
| Input/focus: 009BC6D8, 009BC705, 009BC75D, 009BC79D, 009BD1CD, 009BD1F6, 009BE862 | Manager/capture, focus, timer delivery and callback/reentrancy behavior; section 10 specifies owner gates and calls, not foreign delivery outcomes. |
| Flags/font/gutters: 009BC7DD, 009BC81C, 009BC904, 009BCA09, 009BFCEF, 009BFDC6, 009BFE03 | Base geometry/flag effects, foreign font/style reference and metric outcomes, recalc/bar callbacks. Do not promote foreign implementations from owner ordering. |
| Layout: 009BCBC5, 009BCC0E, 009BD19B, 009BE179, 009BE239, 009BE7AF, 009BF3E0, 009BF98B | Window dimensions/bar thickness, font prefix/wrapping/height and service callbacks remain controlled inputs; progress and invalid/incoherent array domains are not universal recovery promises. |
| Raw/history: 009BCA39, 009BCA6B, 009BCA9D, 009BF8A8, 009C027E, 009C0396 | Wrapper/replay call contracts plus section 6's generated storage/copy/failure boundary; allocator and string-service nonreturn remain external inputs, without local rollback. |
| Lifecycle/messages: 009BD003, 009BD0E1, 009BFB9C, 009BFF52 | Owner cleanup/forwarding order is present; base lifecycle, timer cancellation/late delivery and external callbacks/allocations are not specified here. |
| Composition/filter/search: 009BE614, 009BF335, 009BE96A | TE5 encoding supplement and TE8 adapters now specify UTF16/Thai, fixed-count padding and exact byte-search edges. Valid-range and environment code-page boundaries still apply. |
| Paint/key: 009BEA28, 009C0A95 | Owner paint/key branches and byte/cache behavior are present; font raster/draw/clip, base conversions, dispatch/clipboard service and foreign composed-motion outcomes remain boundaries. |

No P/B row is promoted merely because it was inventoried, renamed or cited
by another method. Eight additional attributed thunks/helpers have their
small body contracts documented; eight retain bounded roles, including
generated-action failure/copy scope, in text-edit-attribution.md. The folded
maximum setter is specified but remains outside the actual 150-body set.
TE5/TE8 are now answered by those contracts, rather than by the zero-U
count. External service implementations and execution remain outside this
owner-side specification; no aggregate authorizes whole-owner coverage.

- [x] Trace largest actual primary methods first: key handling 2938B, paint
  1850B, member initialization 471B, scrollbar sizes 454B, raw insertion
  447B, high-level insertion 437B, AddScrollBar 413B, SetText 399B.
- [x] Separate byte text, selection order, editable/overwrite flags and caret.
- [x] Establish clipboard gates through injected calls without live clipboard.
- [x] Publish font/window/service contracts and independent vectors.
- [x] Track exact direct-label address/size inventory without inventing a suffix.
- [x] Reconcile the requested 19,999B attribution using actual coverage-owner
  output, exact address/size join and primary helper/vtable evidence (TE8).
- [x] Prove raw-mutation domains, history eviction, mixed password coordinates
  and owner resource/callback failure semantics (TE3/TE4/TE6/TE7).
- [x] Publish a method-specific contract for each direct entry, including
  exact label/signature corrections and valid/unsafe domains; TE1 answered.
- [x] Complete TE5 conversion/composition and TE8 generated-helper
  failure/copy/search contracts, preserving foreign-service boundaries.
- [ ] Prove foreign-owner implementations; a bounded body is not evidence
  of those dependencies or complete-owner execution.
- [ ] Implementation, conformance execution, shadow/game verification and
  replacement: none performed or claimed by this RE-only lane.
