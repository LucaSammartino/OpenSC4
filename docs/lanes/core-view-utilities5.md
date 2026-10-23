| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities5 (SC4ViewUtilities owner closure) | Codex / `core-view-utilities5` | in progress | Audit existing module against current spec; close all settled behavior and conformance coverage |

## Handoff
- Goal: close SC4ViewUtilities in portable core only; no ABI/plugin changes.
- Existing module found at engine/core/src/view_utilities; includes prior inventory and VU12/VU13 numeric limitations.
- Done: reserved detached worktree confirmed clean; read lane rules and inventory outline.
- Next: audit current spec sections and existing implementation/tests, then add Owner closure table and missing settled behavior.
- Source: docs/specs/view-utilities.md; open questions VU12/VU13, index highest VU14.
- No game requests; core-only scope.
- Required final gate: qbuild.py --wt <WT> all, once stable.
