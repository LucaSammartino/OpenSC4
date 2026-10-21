# html-document: open questions
<!-- Questions about [html-document.md](../html-document.md). Append new entries at the END; when one is answered, move it to answered/html-document.md. Rules: README.md. -->

<!-- from: re-html-core-answers4 (2026-10-21) -->
- **HD7 - §7.11 FONT SIZE out-of-object read:** SIZE values such as `--4` (index 7) or indexes whose offset (0x1a8+4*i) mod 2^32 is 0x1c4 or more make the original read heap memory outside the 0x1c4-byte document; the value is not defined by the binary. What should a reimplementation use? [treat as size 0, like an unset table entry]
