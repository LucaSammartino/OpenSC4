# view3d-window: open questions
<!-- Questions about [view3d-window.md](../view3d-window.md). Answered entries move to answered/view3d-window.md. -->

No open questions.

<!-- from: core-view3dwin11 (2026-10-24) -->
- **VW19 — §6k/§6l trigger loading:** When reading saved trigger pairs, does DoLoad pre-size the hash table, insert at bucket heads in stream order, or restore stream chain order? §6l settles writing and live insertion but does not specify reconstruction, including duplicate keys. [Literal insertion in stream order, updating an existing key without growth; save traversal uses the resulting table, so colliding pairs can reverse on load.]
