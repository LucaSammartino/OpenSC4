# sc-resources: open questions

<!-- Questions about [sc-resources.md](../sc-resources.md). Answered entries move to answered/sc-resources.md. -->

No open questions. SCR1–SCR5 and SCR7 were answered on 2026-10-23 by
re-scres-scr5; see [answered entries](answered/sc-resources.md).

<!-- from: glue-scres-shadow5 (2026-10-24) -->
- **SCR8 - native capture ABI for sections 3-8:** Please publish complete x86 signatures (including return channels and concrete versus interface views) for command/file parser input and output tokens, token-record initialization and typed getters/setters, text-adapter string/definition writes, buffer-resource read/write and buffer access, and ASCII writer fields/graph/stream access. The behavior/slot inventories do not establish these argument types. In particular the text numeric writers have only one stack word even for f64/s64: are these const references? What non-mutating accessor can capture the original raw record bytes and ASCII output without changing its cursor or invoking serialization again? [Keep those capture paths off; implement only the specified adapter numeric/descriptor/key reads, with SDK read/seek operations restoring the original position. Partial adapter coverage must not credit the whole subsystem.]
