| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-html-core-answers4 (answer HD5/HD6 in docs/specs/html-document.md) | `re-html-core-answers4` / `.claude/worktrees/re-html-core-answers4` | ready: HD5, HD6 answered from native bytes (spec c8266782); HD7 filed | core html_document: apply §5.1 reset rule, SELECT/SCRIPT returns, FONT SIZE index and RZConvertToUint32A rules; answer HD7 (design choice) |

## Report
Spec-only lane (docs/specs). REA not used; all facts from re_query.py asm and decomp/cGZWinHTML.c.

- **HD5 (§5.1)** DIFFERS: ParseSGMLElements (00908122) sets position 0x194 = 0 after the source copy
  (00908134) and again before ParseTextDocument (0090817e); push-back 0x28, token 0xe8, root and styles are not
  reset. Worked case: a reused object whose previous parse ended in push-back (no `</HTML>`) dispatches the
  new text on the stale token 1 -> plain text. Return = callee result. In game cGZWinHTML::InternalSetDocument
  makes a fresh document per load, so this never occurs there.
- **HD6a (§7.3)** DIFFERS: ParseSELECT returns true except when the closing SkipUntilToken(/SELECT) hits the end
  (false); ParseFORMCTRL tail-jumps to it. Examples added.
- **HD6b (§7.1)** MATCHES: ParseSCRIPT returns SkipUntilToken's result.
- **HD6c (§7.11)** DIFFERS: no clamp to 0. `+`/plain keep negative indexes, `-` has no upper cap (`--4` -> 7);
  read at (0x1a8 + 4i) mod 2^32 (-1 -> 0x1a4, 2^31 -> entry 0). Table of cases added.
- **HD6d (§7.11)** DIFFERS: RZConvertToUint32A picks base 16 for `0x` or a hex letter before any non-digit,
  else 10, then CRT strtoul (whitespace, sign negates mod 2^32, overflow saturates, `-overflow` -> 1). Examples.
- **HD7 filed**: value for out-of-object FONT SIZE reads (heap contents; undefined in the binary).

Commits: c8266782 (spec + questions + QUESTIONS.md index).
