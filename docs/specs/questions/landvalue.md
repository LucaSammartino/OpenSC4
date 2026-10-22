# landvalue: open questions
<!-- Questions about [landvalue.md](../landvalue.md). Append new entries at the END; answered entries move to answered/landvalue.md. -->

<!-- from: coordinator train65 (2026-10-22) -->
- **LV2 - verification of LV1 per-Step capture:** train65 has71/72 exact, versus train64 72/72. Pass43 at2029-07-07 has5 intrinsic,2 total and1 slope cell differences, mean altitude270.05667/270.05804, settled=0. An earlier SimNewDay snapshot reproduces native intrinsic/mode/mean but not total. Does capturing inputs at each original Step read plus the per-developer loaded previous map (LV1) restore this pass? Preserve the trace and compare the actual read-boundary inputs; do not adjust algorithms, sampling or expected outputs. See [incident](../../incidents/train65-landvalue-timing.md).
