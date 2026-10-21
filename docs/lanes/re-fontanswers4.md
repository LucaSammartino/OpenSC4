| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fontanswers4: answer drawing-fonts FN1-FN6 | `re-fontanswers4` / (RE spec lane) | ready: FN1-FN6 answered in docs/specs/drawing-fonts.md sections 6, 8.2, 10, 11, 12 (949e9ac8, 760470a5, 0b8cdd83), archived to questions/answered/drawing-fonts.md; no new questions; REA not used (static export + capstone on bin/SimCity4.exe) | 1. Audit cGZFontSys slots 0, 2, 4, 12, 17-19, 21-24 and AddSubstitution (listed in spec section 1). 2. List all 64 cGZFont slots, then credit cGZFont/cGZFontSys. 3. cGZDrawFontRenderer (vtable 0x00AC14BC). |

## Report

Evidence: `tools/re_query.py` export plus `build-runs/xdis.py` (capstone + pefile over bin/SimCity4.exe) for
bytes outside Ghidra functions (CRT log10 body, vtables, constants). REA was not queried.

Answers (all in docs/specs/drawing-fonts.md):
- FN1 (section 10) DIFFERS: L at 0x009F2160 is CRT log10 (x87 FLDLG2/FYL2X 0x009F221F; SSE2 0x009FE548 uses
  log10(2) at 0x00AEAE60). k = (sharpness + 1)^3.321928: k = 10 at sharpness 1.
- FN2 (section 11) DIFFERS in detail: full UTF16GetThaiCharType table (0x0091169F, classes 1-17) and four ordered
  rules; tone marks / U+0E4C add 2 when class(prev) < 10; one-step carry of form 1 through below vowels. Worked examples.
- FN3 (section 12) DIFFERS in part: name score 0/2/0x20, kind score 4 only for an empty list, style +0/+8/+0x10,
  strict less (ties keep the earlier path), substitution walk, non-empty catalogue always yields a font.
- FN4 (section 12) MATCHES with additions: cGZFontSys slots 5/7/8/9 identified (code page +0x20, default aa +0x14,
  renderer +0x24, mono range +0x18/+0x1C and the per-family map +0x64 insert-unique/erase); the per-family lookup
  runs before Init and so uses the font's empty name.
- FN5 (section 8.2) DIFFERS from the bracket: CalculateWidthOfLines (slot 44) binary search, returns lo*10 or 0;
  slot 45 CalculateTextArea(s, n, rect, minW, maxW, maxH) written as five steps with a worked example.
- FN6 (section 6) DIFFERS in part: probe table; 1250 → U+0142 only, 1257 → U+0160 only; 8/0x10 and unlisted
  code pages return true unprobed.
- FontFixedInfo layout confirmed (+0 font list, +4 family, +0x18 style, +0x1C kind, +0x20 source type).

Coverage: no owner newly exact. cGZFont and cGZFontSys stay partial; the unaudited slots are named in section 1.
Questions: none filed. ABI: no change; no abi-rows file.
