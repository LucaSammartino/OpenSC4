# html-document: open questions
<!-- Questions about [html-document.md](../html-document.md). Append new entries at the END; when one is answered, move it to answered/html-document.md. Rules: README.md. -->

- **HD1** §3 DumpUnhandledAttributes: does the per-entry walk (008ff7e7) have any observable effect in the shipping build (log output, state)? [current: none; reimplement as a no-op]
- **HD2** §5.4 byte 0x154: ParseTextDocument sets it to 1 before inserting the plain text, Init clears it. Is it the preformatted/no-wrap flag read by cHTMLTextSegment, or something else? [current: treat as "preformatted", only set by ParseTextDocument and the PRE path]
- **HD3** §3 ParseAttributeList: after taking a name/value, there is a step that removes a leading and trailing `"` from one of the two strings when both ends are quotes (009056da, around the MakeUpper call). Which string (name or value) does it apply to? [current: unverified; values from quoted syntax never carry quotes anyway]
