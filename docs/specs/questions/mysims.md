# mysims: open questions
<!-- Questions about [mysims.md](../mysims.md). Append new entries at the END; when one is answered, move it to answered/mysims.md. Rules: README.md. -->

MS18 and MS20 are answered in the archive and specified in
mysims.md §4.6.2–§4.6.5, §7.2.2–§7.2.3 and §12.29.

<!-- from: glue-mysimdispatch-shadow9 round 2 (2026-10-24) -->
- **MS21 — route partition and current-marker input (§4.6.3–§4.6.4):** specify the exact interpretation and updates of the two persisted leg indices, including which index selects the route-ready marker, the scan/advance rules on next-leg dispatch and jumpToNextSwitch, and the ordered network-snap arguments/results for an unsnappable zero-length non-FE segment. The new text establishes the decisions and masks after a skip and says partitioning skips equal-mode markers, but does not give executable index/snap rules. A worked deque with index values before/after success, FE rejection, and exhaustion would close this gap. [The core accepts an explicit current marker, implements specified decisions, and returns UnsupportedMs18 on a failed leg requiring an uncaptured partition/skip. It does not delegate dispatch-owned selection to a foreign service or infer the marker from native outputs. Public ABI observation remains the explicit limitation answered by MS20; no request to invent an IID slot.]

<!-- from: glue-mysimdispatch-shadow9 round 2 (2026-10-24) -->
- **MS22 — omitted constructor auxiliary fields (§7.2.3, §6.2.1):** are auxiliary flag +10, auxiliary occupant +FC and auxiliary scalar +100 assigned by the constructor, and if so to what values? MS20 lists explicit scalar/location defaults but does not promise these serialized fields. [The constructor shadow excludes them and the variable tail reference encoding: it compares 208 guaranteed bytes through the four locations, with the unpromised tuning/opaque/leg-index/retry words excluded. Core zero/null placeholders remain deterministic storage only, not claims about native pre-Init bytes.]
