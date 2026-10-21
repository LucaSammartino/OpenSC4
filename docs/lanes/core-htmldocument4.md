| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-htmldocument4 (clean-room Rust core of cHTMLDocument per docs/specs/html-document.md) | `core-htmldocument4` / `.claude/worktrees/core-htmldocument4` | ready: engine/core/src/html_document implements all 70 named methods + 21 unnamed owner helpers; 14 conformance tests; full gate PASS (cargo 3994/0, clippy 0, ctest 65/65); questions HD5, HD6 | glue lane: wire into plugin when cGZWinHTML is reimplemented; answer HD5/HD6 |

## Report

**What changed** (Rust core only; no plugin/CAPI/CMake/tools edits; ABI unchanged):
- `engine/core/src/html_document/mod.rs`: `HtmlDocument<H: HtmlHost>`, element arena (`Node`/`Kind`), `HtmlHost`
  trait (GS colours, `SetColor`, table colour, downloader, image loader, temp files, bullet surfaces, anchor name
  test, `HTMLIsSpace`/`HTMLAdvance` with SBCS defaults), tag table (160), grammar sets, constructor/Init/
  SetBaseURL/font-size table get+set/QI/Shutdown/Drop/FindNamedAnchor/slot 4/15, ParseCDATA, GetToken,
  SkipUntilToken, SplitTag, StripWhiteSpace, ParseAttributeList (quote-name strip quirk, first wins, empty name
  dropped), FindAttribute, DumpUnhandledAttributes (no-op), style/alignment stacks, InsertTextSegment, image
  request slot 16, DoMessage, LoadFromImageFile, ResourceHasBeenDownloaded (bytes -> temp file -> load -> remove;
  path case unchanged), CreateBullet with process-wide cache, MakeOutputStyle / MakeRelativeToTextAlignmentStyle,
  the §5.4 segment whitespace rule (`normalize_segment_text`).
- `parse.rs`: entry points (§5), head (§6, §7.1, §7.13), FLOW/INLINE/SPECIAL/FORMCTRL/BLOCK dispatch, BR/INPUT/
  SELECT/OPTION, DIV/CENTER, FORM, BLOCKQUOTE, P, PRE, DL/DT/DD, H1-6, UL/OL/LI, HR, BODY, TABLE/COLGROUP/THEAD/
  TFOOT/TBODY/TR/TD/TH, FONTSTYLE/FONT, PHRASE, SPAN (CSS subset incl. font-size no-op), BLINK, A, IMG, MAP, AREA.
- `entities.rs`: GetStringForEscape table (100 entries, case-sensitive, `?` on miss).
- Element-class setters owned elsewhere (SetPixels, SetLength, SetWIDTH, SetNOWRAP) receive the raw attribute
  bytes as typed fields; no native document is called or adopted.

**Measured:** `qbuild all` PASS: cargo test --workspace 3994 passed / 0 failed, clippy 0 diagnostics, CMake build
35/35, ctest 65/65. html_document tests 14/14 (spec worked examples: tokens, attributes, mis-nesting, forms,
select, tables incl. `</colgroup>` edge, lists/OL numbering/serials, bullets 11x9, headings, PRE, HR, A/SPAN/FONT
styles, IMG/MAP, downloads byte vs path, entities, alignment keywords).

**Whole-owner:** all 70 named methods and the 21 unnamed owner helpers are implemented; nothing remains.

**Questions:** HD5 (position/push-back reset in ParseSGMLElements), HD6 (ParseSELECT/ParseSCRIPT return values,
negative FONT size index, RZConvertToUint32A details) in docs/specs/questions/html-document.md.

**Game:** n/a (core-only scope; no plugin wiring, so no shadow).

**Next:** a glue lane can host this behind cGZWinHTML once that owner is reimplemented; adjust to HD5/HD6 answers.
