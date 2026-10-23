# sc-resources: open questions

<!-- Questions about [sc-resources.md](../sc-resources.md). Answered entries move to answered/sc-resources.md. -->

<!-- from: re-persist-sl5 (2026-10-23) -->
- **SCR1 (§2):** What are the exact term/function grammar and remainder rule,
  and mixed-delimiter nesting/escape behavior? Enum lookup and color/vector
  contracts are now specified in §2.3. Inspect `0x0067AC30`,
  `0x0067B990`; verify numeric
  operations against instructions before publishing constants/rounding.
  [unsettled; only operator precedence and binary64 accumulation specified]
<!-- from: re-persist-sl5 (2026-10-23) -->
- **SCR2 (§3):** Complete cCommandParser method-name/default-state audit,
  built-in command argument/failure contracts and variable insertion boundary
  parentheses. Anchors `0x00675F50`, `0x00677B90`, `0x00678B00`.
  [unsettled; published tables retain unknown semantic names]
<!-- from: re-persist-sl5 (2026-10-23) -->
- **SCR3 (§4):** Settle include relative-path resolution and recovery after
  nested parse errors. Watcher message, slot 11 and include literal/open-error
  policy are now specified in §4. [unsettled]
<!-- from: re-persist-sl5 (2026-10-23) -->
- **SCR4 (§§5–6):** Complete token wire type-number table
  and binary adapter boundary. Descriptor comparison, QI/defaults and the
  final-byte counter rule are now specified in §§5–6. [type 3 is uint32;
  other type assignments and binary field payload encoding unsettled]
<!-- from: re-persist-sl5 (2026-10-23) -->
- **SCR5 (§7):** Settle record BMP offset handling and exact 4444 dithering
  arithmetic/rounding (`0x0097822A`, `0x0097844B`, `0x00978E35`). Failed-load
  retained-buffer state and malformed RLE limits are not currently defined.
  [unsettled; use the documented supported conversion pairs only]
