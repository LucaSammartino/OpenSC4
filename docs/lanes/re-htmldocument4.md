| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-htmldocument4 (RE spec of exact owner cHTMLDocument, 70 fns / 38220 B) | `re-htmldocument4` / (RE spec lane) | ready, round 3/3: HD1-HD4 answered statically (no open questions); named owner 70/70 (38220 B) plus 21 unnamed owner methods (2586 B) specified; whole-owner credit claimable. No engine/ABI/tool edits. | 1. Optional: let a coverage-map lane adopt the 21 unnamed methods (spec §10.3 list) into cHTMLDocument. 2. Element classes (cHTMLTextSegment etc.) need their own spec; §5.4 gives FixTextSegmentWhitespace rules. |

## Report (round 3)

Commits: c8b595b6 (HD1-HD4), 6ab3d3f2 (§7.14 + §10.3), final commit (§1.6 slots/teardown, report).

- HD1 MATCHES: DumpUnhandledAttributes only walks a map iterator begin->end (008ff7e7/008ff123/008fe87e pure); no output, args unused. Spec §3.
- HD2 MATCHES (refined): 0x154 is style-record field +0x24 (record 0x130..0x157), saved/restored by push/pop; text-segment ctor copies it (00908ce2) and passes it to FixTextSegmentWhitespace (00908ea5): 0 collapses whitespace runs to one space, 1 keeps text and maps CRLF/CR/LF to LF. Set by ParseTextDocument and PRE. Spec §5.4 with examples.
- HD3 DIFFERS: the strip applies to the NAME, as erase(1,1)+erase(len-1,1); only the 1-char name `"` qualifies and becomes empty; empty names are not inserted (00905902). Spec §3 steps 6-7, example.
- HD4 MATCHES: +0x2c==0 -> +0x30 bytes swapped (0096edf7) into a local and written verbatim to a MakeTempPathName file opened 00919b00(2 write,1 create-always,0 exclusive), loaded, removed; +0x2c!=0 -> +0x30 is the path. Spec §9.
- Audit (§10.3): 21 unnamed cHTMLDocument methods found (P, PRE, DL, DT, DD, H1-6, list dispatch, UL, OL, CENTER, image request slot 16, COLGROUP, style push/pop, deleting dtor, dtor body, Shutdown 00900357, slots 4/6/13/15), all specified (§1.6, §7.14, §7.8, §8.2). Slots 5/12/14 are shared stubs; container templates and element ctors classified as non-owner.

Evidence: re_query asm/callers/xrefs, decomp/ (cHTMLTextSegment.c, _unnamed/region_00900000.c), exe bytes (build-runs/s.py heading tables 00acd4bc/d4/ec), build-runs/off.py instruction greps. REA not used. Questions: none open.

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
