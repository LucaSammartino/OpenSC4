# legacy-control: open questions
<!-- Questions about [legacy-control.md](../legacy-control.md). Append new entries at the END; when one is answered, move it to answered/legacy-control.md. Rules: README.md. -->

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC8 - sections 7.6–7.7:** what words does each of the 19 per-type script-fragment writers (0x0095F4EC ... 0x009647FB) append after the common words? [partial 2026-10-23 re-legacy-control5b: section 7.7 inventories confirmed output words for all 19 and option child callback words, plus bare-autofit and quote behavior. Exact emission conditions, all value sources and repeated-record formats remain open; no empty-suffix equivalence or full round-trip claim is allowed. Next: trace getter-success gates and metadata reads per writer, including optionswalk 0x0095F87B and Grid/TreeView record iteration.]

<!-- from: close-legacyctl29 (2026-10-26) -->
- **LC17 — §3 control-area table identity:** the specified base `0x00AD5344` plus eight slots places slot 7 at `0x00AD5360`, which §3 also assigns to primary QueryInterface (`0x0094C431`), rather than the stated stub `0x0092B33A`. The stated half-open extent to `0x00AD5360` holds only seven slots. Please resolve the base/extent/slot numbering and confirm the area slots' word addresses. The shadow leaves this table unpatched; no native mismatch or alternate layout is inferred.

<!-- from: close-legacyctl29 (2026-10-26) -->
- **LC18 — §8 comment-list observation for §6.2 replay:** §8 specifies eight strings, a 20-byte private list with vtable `0x00AD5420`, and a variant stored under `0xBACA6879`, but gives no reader interface or field layout. Please specify a safe way to observe the eight strings and the temporary list/variant allocation and reference effects. Is there a queryable public interface, or a published private storage contract? The recorder cannot assume this object implements SDK `cIGZUnknownList`; first-pass Common and whole-build completeness remain zero while this observation is unsupported.
