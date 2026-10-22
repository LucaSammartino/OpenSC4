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
| 0x0091880E | 26 | GetTextLength | U; TE8 |
| 0x0099357F | 7 | GetTextGutterX | U; TE8 |
| 0x00993586 | 7 | GetTextGutterY | U; TE8 |
| 0x00994A39 | 7 | GetText | U; TE8 |
| 0x00994E6A | 7 | GetMaximumTextToAllow | U; TE8 |
| 0x00994E7E | 7 | GetFirstVisibleLine | U; TE8 |
| 0x00994E85 | 7 | GetLineHeight | U; TE8 |
| 0x00994E8C | 45 | SetText | P; sections 2-5 |
| 0x00994EB9 | 7 | GetCaption | U; TE8 |
| 0x00994EC0 | 23 | SetCaption | U; TE8 |
| 0x00994ED7 | 13 | SetMaxUndoLevelCount | P/B; sections 6-7; TE3/TE4 |
| 0x00994FF2 | 10 | GetLineCount | U; TE8 |
| 0x009BC5E1 | 45 | QueryInterface | P; section 1 (constructor sequence bounded) |
| 0x009BC60E | 39 | ToggleCaretVisibility | P; sections 2-5 |
| 0x009BC635 | 39 | MakeCaretVisible | P; sections 2-5 |
| 0x009BC65C | 89 | StartCaretTimer | P; sections 2-5 |
| 0x009BC6B5 | 35 | GZOnKeyUp | U; TE8 |
| 0x009BC6D8 | 45 | GZOnMouseDownR | P/B; section 10; TE7 |
| 0x009BC705 | 88 | GZOnMouseWheel | P/B; section 10; TE7 |
| 0x009BC75D | 64 | GZOnSetFocus | P/B; section 10; TE7 |
| 0x009BC79D | 64 | GZOnKillFocus | P/B; section 10; TE7 |
| 0x009BC7DD | 63 | SetFlag | P/B; section 10; TE7 |
| 0x009BC81C | 27 | SetFlag | P/B; section 10; TE7 |
| 0x009BC837 | 29 | ClearFlag | U; TE8 |
| 0x009BC854 | 16 | IsFlagSet | U; TE8 |
| 0x009BC864 | 42 | GetScrollBarImageVertical | U; TE8 |
| 0x009BC88E | 42 | GetScrollBarImageHorizontal | U; TE8 |
| 0x009BC8DB | 41 | SetFillColor | U; TE8 |
| 0x009BC904 | 74 | SetFontStyle | P/B; section 10; TE7 |
| 0x009BC94E | 25 | GetFontStyle | U; TE8 |
| 0x009BC967 | 20 | SetColor | U; TE8 |
| 0x009BC97B | 22 | GetColor | U; TE8 |
| 0x009BC991 | 27 | GetBackgroundImage | U; TE8 |
| 0x009BC9AC | 93 | SetBackgroundImage | P/B; section 10; TE7 |
| 0x009BCA09 | 48 | SetTextGutterY | P/B; section 10; TE7 |
| 0x009BCA39 | 50 | InsertText | P/B; sections 6-7; TE3/TE4 |
| 0x009BCA6B | 50 | RemoveText | P/B; sections 6-7; TE3/TE4 |
| 0x009BCA9D | 97 | RemoveText | P/B; sections 6-7; TE3/TE4 |
| 0x009BCAFE | 22 | SetInsertMode | P; sections 2-5 |
| 0x009BCB14 | 12 | GetInsertMode | U; TE8 |
| 0x009BCB2E | 23 | GetInsertionLineAndColumn | U; TE8 |
| 0x009BCB45 | 88 | SetCaretPeriod | U; TE8 |
| 0x009BCB9D | 17 | GetCaretPeriod | U; TE8 |
| 0x009BCBAE | 23 | GetSelection | U; TE8 |
| 0x009BCBC5 | 73 | CalculateVisibleWidth | P/B; section 8; TE5 |
| 0x009BCC0E | 73 | CalculateVisibleHeight | P/B; section 8; TE5 |
| 0x009BCC57 | 29 | SetSoundID | U; TE8 |
| 0x009BCC74 | 31 | StartScrollTimer | U; TE8 |
| 0x009BCC93 | 28 | StopScrollTimer | U; TE8 |
| 0x009BCCAF | 31 | ScrollTimerCallback | U; TE8 |
| 0x009BCCCE | 236 | DoCursorScroll | U; TE8 |
| 0x009BCDBA | 131 | RemoveScrollBar | U; TE8 |
| 0x009BCE3D | 454 | SetScrollBarSizes | U; TE8 |
| 0x009BD003 | 172 | Shutdown | P/B; section 10; TE7 |
| 0x009BD0E1 | 139 | DoMessage | P/B; section 10; TE7 |
| 0x009BD16C | 47 | CaretTimerCallback | U; TE8 |
| 0x009BD19B | 50 | GetTextArea | P/B; section 8; TE5 |
| 0x009BD1CD | 41 | GZOnMouseUpL | P/B; section 10; TE7 |
| 0x009BD1F6 | 130 | GZOnMouseMove | P/B; section 10; TE7 |
| 0x009BD278 | 129 | SetScrollBarImageVertical | U; TE8 |
| 0x009BD2F9 | 129 | SetScrollBarImageHorizontal | U; TE8 |
| 0x009BD37A | 31 | SetCaretColor | U; TE8 |
| 0x009BD399 | 32 | GetCaretColor | U; TE8 |
| 0x009BD3B9 | 7 | GetFont | U; TE8 |
| 0x009BD3C0 | 33 | SetFontColor | U; TE8 |
| 0x009BD3E1 | 37 | SetFontColor | U; TE8 |
| 0x009BD406 | 65 | GetFontColor | U; TE8 |
| 0x009BD447 | 56 | ClearFontColor | U; TE8 |
| 0x009BD47F | 22 | GetHighlightFillColor | U; TE8 |
| 0x009BD495 | 31 | SetHighlightFillColor | U; TE8 |
| 0x009BD4B4 | 48 | SetOutlineUse | U; TE8 |
| 0x009BD4E4 | 44 | GetOutlineUse | U; TE8 |
| 0x009BD510 | 68 | GetIndexFromPosition | U; TE8 |
| 0x009BD554 | 63 | GetCharacterCountOfLine | U; TE8 |
| 0x009BD593 | 125 | GetLineAndColumnFromIndex | U; TE8 |
| 0x009BD610 | 94 | GetIndexFromLineAndColumn | U; TE8 |
| 0x009BD66E | 70 | SetInsertionIndex | U; TE8 |
| 0x009BD6B4 | 102 | SetInsertionLineAndColumn | U; TE8 |
| 0x009BD71A | 224 | IncrementInsertionPositionByComposedCharacter | U; TE8 |
| 0x009BD7FA | 163 | IncrementInsertionPosition | U; TE8 |
| 0x009BD89D | 250 | IncrementInsertionPositionByWord | U; TE8 |
| 0x009BD997 | 91 | SetSelection | P; sections 2-5 |
| 0x009BDA65 | 155 | FixInsertionPositionIfBroken | U; TE8 |
| 0x009BDB00 | 131 | GetPixelWidthOfLongestLine | U; TE8 |
| 0x009BDB83 | 334 | GetHorizontalLineMetrics | U; TE8 |
| 0x009BDCD1 | 241 | SetScrollBarValues | U; TE8 |
| 0x009BDDC2 | 48 | GetHorizontalScrollPixels | U; TE8 |
| 0x009BDE57 | 48 | ScrollLineUp | U; TE8 |
| 0x009BDE87 | 66 | ScrollLineDown | U; TE8 |
| 0x009BDEC9 | 112 | ScrollLeft | U; TE8 |
| 0x009BDF39 | 142 | ScrollRight | U; TE8 |
| 0x009BDFC7 | 66 | ScrollToStart | U; TE8 |
| 0x009BE009 | 90 | ScrollToEnd | U; TE8 |
| 0x009BE063 | 70 | ScrollPageUp | U; TE8 |
| 0x009BE0A9 | 104 | ScrollPageDown | U; TE8 |
| 0x009BE111 | 104 | ResizeWindowForExactLineHeights | U; TE8 |
| 0x009BE179 | 192 | GetColumnFromPosition | P/B; section 8; TE5 |
| 0x009BE239 | 162 | GetPositionFromLineAndColumn | P/B; section 8; TE5 |
| 0x009BE2DB | 39 | GetText | U; TE8 |
| 0x009BE302 | 373 | PutInsertionPositionInView | P/B; section 8; TE5 |
| 0x009BE477 | 413 | AddScrollBar | P/B; section 8; TE5 |
| 0x009BE614 | 186 | FixBadCharacterComposition | U; TE8 |
| 0x009BE6CE | 225 | SelectCurrentWord | U; TE8 |
| 0x009BE7AF | 179 | TestForPossibleScrollBarAdditionOrRemoval | P/B; section 8; TE5 |
| 0x009BE862 | 227 | GZOnMouseDownL | P/B; section 10; TE7 |
| 0x009BE945 | 37 | UpdateScrollBarsAndWindowPosition | U; TE8 |
| 0x009BE96A | 53 | FindText | U; TE8 |
| 0x009BE99F | 16 | ShowWindow | U; TE8 |
| 0x009BEA28 | 1850 | GZPaint | P/B; section 9; TE6 |
| 0x009BF162 | 387 | GetLineAndColumnFromPosition | U; TE8 |
| 0x009BF2E5 | 80 | AdjustLinePositionArray | U; TE8 |
| 0x009BF335 | 171 | RemoveBadCharacters | U; TE8 |
| 0x009BF3E0 | 312 | CalculateAllLineLengths | P/B; section 8; TE5 |
| 0x009BF578 | 63 | RemoveBadCharacters | U; TE8 |
| 0x009BF5B7 | 447 | InsertText | P/B; sections 6-7; TE3/TE4 |
| 0x009BF776 | 306 | RemoveChar | P/B; sections 6-7; TE3/TE4 |
| 0x009BF8A8 | 227 | RemoveText | P/B; sections 6-7; TE3/TE4 |
| 0x009BF98B | 233 | RecalculateAll | P/B; section 8; TE5 |
| 0x009BFB9C | 265 | Init | P/B; section 10; TE7 |
| 0x009BFCA5 | 74 | SetArea | U; TE8 |
| 0x009BFCEF | 215 | SetFlags | P/B; section 10; TE7 |
| 0x009BFDC6 | 61 | SetFont | P/B; section 10; TE7 |
| 0x009BFE03 | 72 | SetTextGutterX | P/B; section 10; TE7 |
| 0x009BFF52 | 28 | ~cGZWinTextEdit | P/B; section 10; TE7 |
| 0x009BFF6E | 471 | InitializeMemberVariables | P; section 1 (constructor sequence bounded) |
| 0x009C01AF | 207 | cGZWinTextEdit | P; section 1 (constructor sequence bounded) |
| 0x009C027E | 280 | Undo | P/B; sections 6-7; TE3/TE4 |
| 0x009C0396 | 280 | Redo | P/B; sections 6-7; TE3/TE4 |
| 0x009C04FE | 399 | SetText | P; sections 2-5 |
| 0x009C068D | 210 | AddUndoRecordForTextInsertion | P/B; sections 6-7; TE3/TE4 |
| 0x009C075F | 130 | AddUndoRecordForTextTextDeletion | P/B; sections 6-7; TE3/TE4 |
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
| 0x009BDE3E | 25 | Unresolved helper; TE8 |
| 0x009BE9AF | 31 | Action destruction helper called by replay; full closure TE7/TE8 |
| 0x009BE9ED | 32 | Creates initial line-start entry; call-site contract section 8 |
| 0x009BEA0D | 27 | Unresolved helper; TE8 |
| 0x009BF518 | 24 | Unresolved helper; TE8 |
| 0x009BFA74 | 30 | History-vector destruction helper; section 10 call order; internals TE7 |
| 0x009BFA92 | 41 | Unresolved helper; TE8 |
| 0x009BFABB | 40 | Unresolved helper; TE8 |
| 0x009BFAE3 | 163 | Nondeleting destructor; section 10 |
| 0x009BFF03 | 68 | Copies action scalar fields, owned string and five-word state; TE8 for full helper domain |
| 0x009C017D | 50 | Forward action copy range with 60-byte stride; TE8 for full helper domain |
| 0x009C04AE | 29 | Unresolved helper; TE8 |

Other folded/unnamed text-vtable entries include 0x009BC8B8, 0x00994F79,
0x009C32AC, 0x009BCB20, 0x009D41A1 and 0x00994EE4. A foreign-looking
folded name does not establish the text slot's meaning. GetTextLength at
0x0091880E actually copies bytes to an output string, whereas GetText at
0x00994A39 returns end-minus-begin; do not derive signatures from labels.
The text vtable has 90 listed entries; the primary window vtable has 156.
Inherited base/stub slots are not additional primary text-editor bodies.

## Whole-owner acceptance checklist

- [x] Trace largest actual primary methods first: key handling 2938B, paint
  1850B, member initialization 471B, scrollbar sizes 454B, raw insertion
  447B, high-level insertion 437B, AddScrollBar 413B, SetText 399B.
- [x] Separate byte text, selection order, editable/overwrite flags and caret.
- [x] Establish clipboard gates through injected calls without live clipboard.
- [x] Publish font/window/service contracts and independent vectors.
- [x] Track exact direct-label address/size inventory without inventing a suffix.
- [x] Reconcile the requested 19,999B attribution using actual coverage-owner
  output, exact address/size join and primary helper/vtable evidence (TE8).
- [ ] Complete U/B method contracts, raw-mutation domains, encoding and
  composition, word navigation, metrics/scrollbar resource/range behavior,
  history eviction and exceptional resource lifetimes (TE1, TE3-TE8).
- [ ] Implementation, conformance execution, shadow/game verification and
  replacement: none performed or claimed by this RE-only lane.

