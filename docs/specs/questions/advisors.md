# advisors: open questions
<!-- Questions about [advisors.md](../advisors.md). Append new entries at the END; when one is answered, move it to answered/advisors.md. Rules: README.md. -->


<!-- from: re-advisor-api4 (2026-10-21) -->
- **AD11 - section 4.2:** what path does `cISC4App` slot 55 `GetSkuSpecificDirectory` (`0x00449E20`, the string object at app `+0xC4`) hold at run time, and who fills it? `PreFrameWorkInit` fills the data, CD, Plugins, Sku_Data and user-data strings but not this one; a runtime read of the getter would settle it. [unknown; the replacement should call the getter, not build the path]
