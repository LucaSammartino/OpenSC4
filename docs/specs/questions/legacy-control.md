# legacy-control: open questions
<!-- Questions about [legacy-control.md](../legacy-control.md). Append new entries at the END; when one is answered, move it to answered/legacy-control.md. Rules: README.md. -->

<!-- from: re-legacycontrol4 (2026-10-21) -->
- **LC8 - sections 7.6–7.7:** what words does each of the 19 per-type script-fragment writers (0x0095F4EC ... 0x009647FB) append after the common words? [partial 2026-10-23 re-legacy-control5b: section 7.7 inventories confirmed output words for all 19 and option child callback words, plus bare-autofit and quote behavior. Exact emission conditions, all value sources and repeated-record formats remain open; no empty-suffix equivalence or full round-trip claim is allowed. Next: trace getter-success gates and metadata reads per writer, including optionswalk 0x0095F87B and Grid/TreeView record iteration.]

<!-- from: glue-legacyctl-shadow12 (2026-10-24) -->
- **LC15 - sections 6 and 9 observable build boundary:** which native callers dispatch SetParameters through the primary vtable, and what published interception boundary also covers direct SetParameters/per-type calls? Please specify calling conventions, input registers/stack, and a safe hook span or return site for that boundary. [choice: patch only the published primary vtable entry; count builds seen there but exclude full property/interpreter replay from the owner-only shadow. No inline patch or inferred instruction boundary.]
