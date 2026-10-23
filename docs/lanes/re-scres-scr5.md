| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-scres-scr5: SC resource open contracts | Codex / `re-scres-scr5` (RE lane) | in progress | Verify and answer SCR1-SCR5 and SCR7; doccheck; commit and enqueue. |

## Handoff

- Goal: close every open SCR question in sc-resources with behavior and vectors.
- Worktree reused, intentionally detached; no branch/ref updates.
- Read rules, lane format, questions and core-sc-resources5 open-boundary report.
- Done: 4d687c7b drafts SCR1; 9128baba drafts SCR4 wire table/SCR5 BMP/dither; NOT yet docchecked or questions moved.
- Next: finish SCR2/SCR3/SCR7 using facts below, add vectors, review drafts, move answers/index; run `python tools/doccheck.py check`, report/enqueue.
- Guard OVER at151k; no queue. Initial doccheck invocation lacked subcommand (usage error); corrected check blocked, so no doccheck verdict.
- All six question entries remain open intentionally until vectors and review are complete; do not report ready.
- Evidence helpers (ignored): build-runs/sections.py excerpts functions by address; peek.py reads PE data (suffix :84w prints vtable slots).
- Type map verified from constructors and Read dispatch: 1u8,2u16,3u32,4s32,5f32,6s64,7string,8key,9s8,10s16,11u64,12f64.
- Binary fixed widths require counter<total after advance; strings delegate first and use <=total; no text fragment unwrapping.
- SCR5: record ignores bfOffBits cursor, checks uint32(offset+14)>=54; zero RLE size uses bfSize-offset; malformed bounds unsafe.
- Dither: integer serpentine FS weights7/3/5/1; nibble residuals, floor /16 then upper clamp255; type7 selects4444. Verified asm 978325-9783df/978571-978605.
- Review existing loader flag prose: record BMP lock is8040 in asm; existing spec says record8080 without distinguishing TGA/creation/lock.
- SCR2 evidence: decomp/nSCRes/cSetCommand.c (67510), cNamespaceCommand.c, cEvalCommand.c; scalar set requires>=3 tokens and uses third token.
- setc local alpha is separate from RGB; scale RGB only, mult/add RGBA, alpha override, output alpha only<1; verify asm before publishing.
- slots7 PushNamespace (677210 appends name and colon, saves old prefix length);8 pop;9 clear variable map(678ab0);12 GetVariable(6772c0).
- GetVariable tries full current prefix+name then saved prefix lengths newest-first, ending unqualified; SetVariable always full prefix.
- substitution $/% accepts embedded names with letters/digits/_/: and optional braces; NOT whole-token only. Arithmetic wrapping depends on boundary charset B46974; find initializer via xrefs (not destructor A7C9A0).
- Parser ctor is split FUN_6788b0/6788b9; asm67893d default version1, maps/stacks/active macro/prefix empty. Read defaults fully next.
- SCR3: 680b20 classifies drive-letter-colon or initial slash/backslash absolute; 6810dd prepends current parent dir otherwise; no canonicalization observed.
- Include nested catch68110f: code2 suppressed by sinclude, include promotes to3; other errors rethrow. 681273 closes and appends filename:line; no resume enclosing file.
- Top-level catch6813a2: optional open code2 ignored; other optional errors recorded but do not force failure; required errors fail; next file still parsed; 6813ed posthook always, success hook only if overall success.
- SCR7 remaining: confirm numeric CRT overflow/nonfinite; strings #INF/#QNAN/#IND/#SNAN at AECFF0-AED008, formatter A04942. Do not infer modern inf/nan spelling.
- Image resource primary table AD8F28 inherits key.type getter at9CEDE1 (+0xC), zero on base construction975633; no hardcoded image base type established beyond key.type.
- Main evidence root: C:/Users/tlori/FreeCity4; edits only in this worktree.
- No engine reads/edits, ABI rows or game runs planned.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-scres-scr5-r1-manifest.json.
