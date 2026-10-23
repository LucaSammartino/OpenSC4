# sc-resources: open questions

Questions about [sc-resources.md](../sc-resources.md). Evidence anchors and
known behavior are in the named sections; these entries isolate remaining
implementation boundaries.

- **SCR1 (§2):** What are the exact term/function grammar and remainder rule,
  and mixed-delimiter nesting/escape behavior? Enum lookup and color/vector
  contracts are now specified in §2.3. Inspect `0x0067AC30`,
  `0x0067B990`; verify numeric
  operations against instructions before publishing constants/rounding.
- **SCR2 (§3):** Complete cCommandParser method-name/default-state audit,
  built-in command argument/failure contracts and variable insertion boundary
  parentheses. Anchors `0x00675F50`, `0x00677B90`, `0x00678B00`.
- **SCR3 (§4):** Settle include relative-path resolution and recovery after
  nested parse errors. Watcher message, slot 11 and include literal/open-error
  policy are now specified in §4.
- **SCR4 (§§5–6):** Complete token wire type-number table, descriptor matching
  and binary adapter boundary. Descriptor comparison, QI/defaults and the
  final-byte counter rule are now specified in §§5–6.
- **SCR5 (§7):** Settle record BMP offset handling and exact 4444 dithering
  arithmetic/rounding (`0x0097822A`, `0x0097844B`, `0x00978E35`). Failed-load
  retained-buffer state and malformed RLE limits are not currently defined.
- **SCR6 (§8):** Audit empty-stack OutputStream behavior,
  array separators, complete graph/reference literals, and surveyor creation
  failure. Existing scalar and graph order facts are not a byte-complete
  ASCII grammar (`0x00982F1C`, `0x009835ED`).
