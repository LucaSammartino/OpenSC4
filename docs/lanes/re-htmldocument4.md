| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-htmldocument4 (RE spec of exact owner cHTMLDocument, 70 fns / 38220 B) | `re-htmldocument4` / (RE spec lane) | wip, round 1 of max 3: docs/specs/html-document.md §1-9 cover 48 of 70 owner functions (24100 of 38220 B); 22 functions (14120 B) remain, so no whole-owner credit. Questions HD1-HD4 filed. Tip 6c7bec5a + report commit. No engine/ABI/tool edits. | 1. Spec ParseBODY, ParseTABLE/TR/TD/TH, ParseLI, ParseDL(BLOCKQUOTE), ParseHR, CreateBullet. 2. ParseFONTSTYLE, ParsePHRASE, ParseA, ParseIMG, ParseMAP, ParseAREA, ParseSPAN, ParseBLINK, ParseMETA, ParseLINK. 3. GetStringForEscape entity table (script it like build-runs/tags.py), 2 hashtable helpers. 4. Then whole-owner credit. |

## Handoff (round 2, in progress)
- Goal: spec the 22 remaining cHTMLDocument functions (14120 B) in docs/specs/html-document.md; whole-owner credit only when §10 remainder is empty.
- Done round 2: §7.7-7.10 (21332e72); §7.11 inline styles, §7.12 IMG/MAP/AREA, §7.13 META/LINK (next commit).
- Next: GetStringForEscape 00901222 entity table (script it), hashtable helpers 008ffdc8/009001e7, then §10 empty -> report.
- Facts: REA unused this round (asm + decomp suffice). build-runs/s.py <va>... prints exe strings (prefix d for dword).

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
