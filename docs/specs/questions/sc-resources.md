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

<!-- from: core-sc-resources5 (2026-10-23) -->
- **SCR7 (§§2–3,5–7):** Please settle the remaining portable boundaries:
  decimal operand/exponent grammar; variable namespace lookup order and `%`
  parameter boundaries; whether SetTokenString names the field definition;
  how length-prefixed SetFieldString fragments are consumed by the raw text
  delimiter reader; runtime numeric overflow/nonfinite spelling; the color-type
  number selecting 4444 and the image resource's base type. [Minimal decimal
  literals; newest namespace prefix first; whole-token `%name`; token string
  used as definition name; preserve literal SetFieldString writes and raw
  delimiter reads separately; numeric prefixes use caller-width wrapping;
  no claimed text roundtrip or 4444 parity; image base type zero pending a
  specified type. Existing explicit open domains remain SCR1–SCR5.]
