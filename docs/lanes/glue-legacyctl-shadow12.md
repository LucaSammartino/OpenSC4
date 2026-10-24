| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-legacyctl-shadow12 (legacy control shadow) | `Codex` / `glue-legacyctl-shadow12` (engine lane) | in progress | Inspect observable boundary contracts; implement and validate supported shadow. |

## Handoff
- Goal: default-off same-process legacy control state shadow, measured through 2032.
- Done: owner-only Rust replay and private static-link transport; default-off legacyctl_shadow vtable observers.
- Coverage: primary QI/AddRef/SetOwnControl, legacy GetControl/SetOwnControl; all five owner fields compared.
- SetParameters counts and post-build GetControl probes are separate; property/interpreter replay excluded.
- Questions: LC14 return contracts; LC15 observable direct-call boundary, in docs/specs/questions/legacy-control.md.
- Next: collect focused cargo session 72034; run qbuild all on stable final source, native verify if not delegated by all; game to 2032.
- Focused check: build-runs/legacyctl-focused.log; no game request outstanding.
- Formatting fixes applied; initial relative-path fmt and q.py Bash permission failure preserved in build-runs.
- Constraints: no native signatures inferred; no prohibited sources; no agents.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-legacyctl-shadow12-r1-manifest.json.
