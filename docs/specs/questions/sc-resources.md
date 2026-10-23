# sc-resources: open questions

Questions about [sc-resources.md](../sc-resources.md). Evidence anchors and
known behavior are in the named sections; these entries isolate remaining
implementation boundaries.

- **SCR1 (§2):** What are the exact term/function grammar and remainder rule,
  mixed-delimiter nesting/escape behavior, ParseEnum unmatched-name policy,
  color scaling/packing and vector component grammar? Inspect `0x0067AC30`,
  `0x0067B990`, `0x0067AAF0`, `0x0067BDE0`–`0x0067C060`; verify numeric
  operations against instructions before publishing constants/rounding.
- **SCR2 (§3):** Complete cCommandParser vtable/IID/default-state audit,
  built-in command argument/failure contracts and variable insertion boundary
  parentheses. Anchors `0x00675F50`, `0x00677B90`, `0x00678B00`.
- **SCR3 (§4):** Which watcher messages are accepted at table `0x00AAF464`,
  what is main slot 11's getter, and what literal include names and failure
  policies initialize globals `0x00B47B24`/`0x00B47B38`?
- **SCR4 (§§5–6):** Complete token wire type-number table, descriptor matching
  (`0x0097EF5B`), QI IID/default count (`0x0097EF84`), binary adapter boundary,
  and final-byte counter vectors (`0x0097EF35`). Does a final delimiter need
  an additional trailing byte to be consumable?
- **SCR5 (§7):** Settle record BMP offset handling and exact 4444 dithering
  arithmetic/rounding (`0x0097822A`, `0x0097844B`, `0x00978E35`). Failed-load
  retained-buffer state and malformed RLE limits are not currently defined.
- **SCR6 (§8):** Audit constructor defaults, empty-stack OutputStream behavior,
  array separators, complete graph/reference literals, and surveyor creation
  failure. Existing scalar and graph order facts are not a byte-complete
  ASCII grammar (`0x00982F1C`, `0x009835ED`).
