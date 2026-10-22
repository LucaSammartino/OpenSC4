| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-overlay-views4: pure Rust overlay owners | `core-overlay-views4` / engine lane, detached worktree | in progress; base 0168a928; no ABI changes | 1. Implement specified owners and tests. 2. Resolve or file exact missing contracts. 3. Run stable final gate. |

## Handoff
- Goal: six exact owners in engine/core/src/overlay_views; no CAPI/plugin/ABI/maps edits.
- Base: detached HEAD 0168a928. Worktree already reserved; do not change refs.
- Read: overlay-views §§1–3, network-overlay-views §§1–2, terrain-view §8; the latter has no declared-owner algorithms.
- Existing questions: OVV1 (repeat-factor source/opaque lot flags); next OVV2, NOV2.
- Main baseline freshly gated; task expressly skips pre-edit full baseline.
- Next: inspect existing terrain_overlays for overlap, ABI checklist, core conformance conventions, then substantive code by call 20–30.
- No game request; pure core has no runtime binding or shadow key.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-overlay-views4-r1-manifest.json.
