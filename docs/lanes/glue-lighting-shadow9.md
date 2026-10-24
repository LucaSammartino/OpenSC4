| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lighting-shadow9: lighting state shadow | Codex, `glue-lighting-shadow9` | in progress | Inspect public contracts and implement monthly comparison. |

## Handoff
- Goal: default-off lighting_shadow, independent Rust expectations, house-safe monthly native comparison through 2032.
- Done: initial handoff a17dafae8; added default-off read-only native slot 14/16(null) comparison of 14 scalar fields to independent shipped-parameter Rust core; monthly deduplication including load month.
- Focused tests: 2/2 lighting_shadow tests pass; explicit-file fmt passes. Initial relative fmt path and Bash q wrapper failed (no source failure); absolute fmt and direct cargo succeeded.
- Open: LT14 in docs/specs/questions/lighting.md requests ordered external-input replay; whole-owner coverage not claimed. Slots 0–13, 15, 17–29 and nonnull-holder slot 16 uncompared.
- Next: stable-source qbuild all, required queued verify, then house-switch turbo run to 2032.
- Constraints: no prohibited source or tools; no agents; no branch changes; shared files additive.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-lighting-shadow9-r1-manifest.json
