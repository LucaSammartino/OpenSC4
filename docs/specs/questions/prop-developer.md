# prop-developer: open questions
<!-- Questions from engine implementers; append new entries at the end. -->

No open questions. PV14–PV17 are archived in [answered/prop-developer.md](answered/prop-developer.md).
No open questions. PV14 and PV15 are archived in [answered/prop-developer.md](answered/prop-developer.md).

<!-- New questions below supersede the earlier status sentence. -->

<!-- from: glue-propdev-close7 (2026-10-24) -->
- **PV18 - section 3.3.1, read-only dated-effect observation:** PV17 specifies the ordinary transition in terms of I/L/S/E, chance, placement and model availability, but does not identify a coherent snapshot route for I/L/S/E (prop-occupant.md section 2.2 identifies only an owned 0x28-byte record at +0x54, and PO4 still leaves its fields unnamed), nor the model-state interfaces and exact calls/releases referred to by "updates any available model state interfaces". Please specify the date-record fields or an existing side-effect-free serialization/accessor route, the separate occupant-generator snapshot location/layout or per-draw observation boundary, and the model-state interface signatures/call ordering needed to compare those outputs independently. Include the held message's AddRef/release lifetime boundary if it is part of the claimed effect trace. Native callback addresses and behavioral transitions alone do not establish safe argument/layout reads. [Current choice: compare scoped night CRT/list/callback outputs under PV16; preserve creation/dated ineligibility until every delegated effect can be observed and compared. Do not infer date-record fields or native model signatures, invoke a native date tick twice, or count replaying an opaque native-effect transcript as independent effect verification.]
