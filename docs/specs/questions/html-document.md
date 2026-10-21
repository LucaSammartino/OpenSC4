# html-document: open questions
<!-- Questions about [html-document.md](../html-document.md). Append new entries at the END; when one is answered, move it to answered/html-document.md. Rules: README.md. -->

<!-- from: core-htmldocument4 (2026-10-21) -->
- **HD5 - §5.1 ParseSGMLElements:** "copy text into the source" does not say whether the parse position (0x194) and the push-back flag (0x28) are reset first. A second ParseSGMLElements on the same object would otherwise start at the old position. [not reset; a fresh object starts at 0]

<!-- from: core-htmldocument4 (2026-10-21) -->
- **HD6 - §7.3 / §7.1 / §7.11 return values:** (a) ParseSELECT's return value is not given; it is observable through ParseINLINE in DT/DD (`ok`). [always true] (b) ParseSCRIPT returns the SkipUntilToken result? [yes] (c) FONT `size=n` with n >= 2^31 gives a negative signed index (read before the table at 0x1a8); what is read? [clamped to index 0] (d) RZConvertToUint32A: leading whitespace / sign / overflow handling? [leading decimal digits only, wrapping]
