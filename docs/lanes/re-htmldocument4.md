| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-htmldocument4 (RE spec of exact owner cHTMLDocument, 70 fns / 38220 B) | `re-htmldocument4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/html-document.md + questions/html-document.md (HD1-HD3) for exact owner cHTMLDocument (70 fns, 38220 B).
- Done: 43d686a6 lane file; 58e6e641 spec §1-6 (lifecycle, tokenizer, 160-entry tag table, attributes, first sets, entry points, head) + §10 remainder list.
- Next: §7 body parsers (ParseBODY 00907e42, ParseBLOCK, ParseTABLE...), §8 style stack/InsertTextSegment, §9 downloads; shrink §10 list as each lands.
- Facts: REA session unavailable ("No app is open") -> asm via re_query; Ghidra drops stores (e.g. push-back flag 0x28 in ParseHEAD) so check asm. Scripts build-runs/tags.py (tag table), sets.py (first sets).
