# sc-resources: open questions

<!-- Questions about [sc-resources.md](../sc-resources.md). Answered entries move to answered/sc-resources.md. -->

No open questions. SCR1–SCR5 and SCR7 were answered on 2026-10-23 by
re-scres-scr5; see [answered entries](answered/sc-resources.md).

<!-- from: glue-scres-shadow5 (2026-10-24) -->
- **SCR8 - native capture ABI for sections 3-8:** Please publish complete x86 signatures (including return channels and concrete versus interface views) for command/file parser input and output tokens, token-record initialization and typed getters/setters, text-adapter string/definition writes, buffer-resource read/write and buffer access, and ASCII writer fields/graph/stream access. The behavior/slot inventories do not establish these argument types. In particular the text numeric writers have only one stack word even for f64/s64: are these const references? What non-mutating accessor can capture the original raw record bytes and ASCII output without changing its cursor or invoking serialization again? [Keep those capture paths off; implement only the specified adapter numeric/descriptor/key reads, with SDK read/seek operations restoring the original position. Partial adapter coverage must not credit the whole subsystem.]

<!-- from: glue-scres-shadow5 (2026-10-24) -->
- **SCR9 - natural resource/token capture dispatch:** In real-city run `scres5-r1-2032-a` (Konradshohe, turbo through 2032-01-19), the specified binary/text adapter vtables installed successfully before PreAppInit, but every numeric/descriptor/key slot had zero observations (73 dated lines, `scres 0/73`, no refusal or native difference). Which natural resource-file paths use these tables, which call implementations directly, and which earlier lifecycle/capture entry is required to see the game's actual parser/token/image/writer traffic? Please specify the complete argument/result ABI and safe passive capture route for those paths. [Try installation at OnStart before framework initialization; never credit an empty sample or substitute constructed inputs for real file traffic.]

  Follow-up evidence (2026-10-24): `scres5-r1-2032-b` installed at OnStart before framework initialization, corrected the ini registration and load date, and reached 2032-01-05 cleanly with no warnings and ref_warnings=0. Still `scres 0/73`: calls=0, bytes=0, refused=0 throughout. Earlier lifecycle installation did not expose natural adapter dispatch. No native value differences were observed because no comparisons occurred. The lane remains WIP without coverage credit.
