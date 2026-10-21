| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-htmldocument4 (RE spec of exact owner cHTMLDocument, 70 fns / 38220 B) | `re-htmldocument4` / (RE spec lane) | ready, round 2: docs/specs/html-document.md now covers all 70 owner functions (38220 B); §10 remainder empty, whole-owner credit claimable. Questions HD1-HD4 still open (none new). No engine/ABI/tool edits. | 1. Answer HD1-HD4 (game/debugger evidence). 2. Optional: spec gap functions P 0090614f, PRE 00906a5d, DL 009066a3, headings 00906754, lists 00906f1d/0090689f/00907d6b, CENTER 00906d22, table-section helpers, image request slot 16 009059fc (separate inventory). |

## Report (round 2)

What changed: `docs/specs/html-document.md` §7.7-7.13 and §10.1-10.2 (commits 21332e72, e95bcfae, 67bffccd).

Specified this round (all 22 remaining functions, 14120 B): ParseBODY (§7.7); ParseTABLE, ParseTR, ParseTD,
ParseTH (§7.8, incl. the fixed CAPTION/COL/THEAD/TFOOT/TBODY order and the `</colgroup>` row-loss edge case);
ParseLI, CreateBullet (§7.9, bullet cache and surface geometry); ParseDL (= BLOCKQUOTE), ParseHR (§7.10);
ParseFONTSTYLE, ParsePHRASE, ParseSPAN, ParseBLINK, ParseA (§7.11); ParseIMG, ParseMAP, ParseAREA (§7.12);
ParseMETA, ParseLINK (§7.13); GetStringForEscape (§10.1, 100-entry table extracted by build-runs/esc.py and
checked equal to HTML 4 Latin-1 + quot/amp/lt/gt); hashtable clear/destructor (§10.2).

Measured: owner remainder list in §10 is empty -> 70/70 functions, 38220/38220 bytes specified.

Notable exact behaviours: FONT SIZE plain n is a 0-based table index; SPAN `font-size` never changes size
(searches px/pt in the property name); SPAN `color:#hex` stores the raw number; IMG without SRC returns false
and ends ParseBODY's loop; TD without ALIGN resets alignment to LEFT; ParseA appends NAME anchors before they
may be deleted (dangling entry); HR ALIGN uses the vertical-placement mapping.

Evidence: re_query asm/callers/callees, decomp/cHTMLDocument.c, exe bytes (build-runs/s.py). REA not used.
Questions: HD1-HD4 open from round 1; none new.

## Report (round 1)

What changed: new `docs/specs/html-document.md` and `docs/specs/questions/html-document.md` (HD1-HD4), QUESTIONS.md regenerated.

Specified (48 of 70 functions, 24100 B): constructor, Init, SetBaseURL, QueryInterface + thunk 009006da,
SetFontSizeTable, InitTagList (full 160-entry tag-id table), BuildFirstSets/AppendSet/IsInSet (9 grammar sets),
ParseCDATA, GetToken, SkipUntilToken, SkipDocumentWhitespace, SplitTag, StripWhiteSpace, ParseAttributeList,
FindAttribute, DumpUnhandledAttributes, ParseFLOW, ParseINLINE, ParseSGMLElements, ParseHTMLDocument,
ParseSGMLDocumentType, ParseTextDocument, ParseHTMLElements, ParseHEAD, InsertTextSegment, ParseTITLE, ParseBASE,
ParseSCRIPT, ParseFORMCTRL, ParseSPECIAL, ParseBR, ParseINPUT, ParseSELECT, ParseOPTION, ParseDIV, ParseDIVActual,
MakeOutputStyle, MakeRelativeToTextAlignmentStyle, ParseBLOCK, ParseP (actually FORM), DoMessage,
LoadFromImageFile, ResourceHasBeenDownloaded.

Remaining (22 fns, 14120 B): ParseMETA, ParseLINK, ParseBODY, ParseDL (actually BLOCKQUOTE), ParseLI, ParseTABLE,
ParseTR, ParseTD, ParseTH, ParseHR, ParseFONTSTYLE, ParsePHRASE, ParseA, ParseIMG, ParseMAP, ParseAREA, ParseSPAN,
ParseBLINK, CreateBullet, GetStringForEscape (6918 B entity table), hashtable clear 008ffdc8, hashtable dtor 009001e7.

Facts for the next round:
- REA returned "No app is open" for every call this round; all evidence is re_query asm/strings, decomp/ and exe bytes.
- Ghidra pseudocode drops stores (push-back flag 0x28 in ParseHEAD, token 1 in GetToken) and braces; check asm.
- Export names ParseP/ParseDL serve FORM/BLOCKQUOTE (ParseBLOCK dispatch + close-tag constants); spec follows tokens.
- build-runs/tags.py (tag table from InitTagList asm + exe strings), build-runs/sets.py (first sets) are local helpers.
