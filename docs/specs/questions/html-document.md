# html-document: open questions
<!-- Questions about [html-document.md](../html-document.md). Append new entries at the END; when one is answered, move it to answered/html-document.md. Rules: README.md. -->

<!-- from: re-html-core-answers4 (2026-10-21) -->
- **HD7 - §7.11 FONT SIZE out-of-object read:** SIZE values such as `--4` (index 7) or indexes whose offset (0x1a8+4*i) mod 2^32 is 0x1c4 or more make the original read heap memory outside the 0x1c4-byte document; the value is not defined by the binary. What should a reimplementation use? [treat as size 0, like an unset table entry]

<!-- from: core-html-document5 (2026-10-23) -->
- **HD8 - §7.7 BODY termination:** the content-loop bullets stop only on EOF or a failed handler and skip every other non-dispatched token, but the worked example says `/BODY` stops the loop without push-back. Should `/BODY` terminate before the following GetToken, preserving `/HTML` for ParseHTMLElements? [follow the explicit loop bullets: skip `/BODY`, stop at EOF or handler failure; this leaves push-back set even after a complete HTML page]

<!-- from: method-html31 (2026-10-27) -->
- **HD9 - §2.1 shared character-helper argument ABI:** what concrete type/layout does the first argument of HTMLIsSpace (0090a929 / 0090a820) and HTMLAdvance (0090a947) accept, how is the index passed/returned, and how can an implementer supply an owned byte copy through a published interface? functions.tsv publishes two cdecl stack dwords but no argument types. In `method-html31-catalog-r2`, passing an SDK `cRZBaseString` as `cIGZString*` to HTMLAdvance caused an AV at 009106d9 reading 4c4d5448; the captured return address is 0090a965. Evidence: `C:/Users/tlori/FreeCity4/build-runs/game-results/method-html31-catalog-r2.log:4020`. No executable or disassembly was inspected. This unsupported adapter has been removed. ParseCDATA, SkipDocumentWhitespace, StripWhiteSpace and SplitTag retain entry counts with complete=0; no ASCII-only fallback or input exclusion is credited. Please publish a compatible interface/factory or a value-level helper contract before those methods resume native comparison.
