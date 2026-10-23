| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lighting-close5: lighting owner closure | Codex, `core-lighting-close5` | in progress | Audit lighting spec and core, close owner coverage, run full gate |

## Handoff
- Goal: close cSC4LightingManager and cImageARGB8 against lighting.md; core only, no ABI/plugin changes.
- Done: audited lighting.md §§0–12 against the existing core; all owner algorithms already implemented. Added section-by-section Owner closure and 22-vector mapping in engine/core/src/lighting/README.md.
- Done: added direct regressions for tint reset, vector 11 capacity/padding, vector 14 missing maps/re-init/shutdown, vector 18 downward/zero vertical basis. Focused lighting suite: 48 passed, 0 failed.
- Done: explicit-file formatting passes using absolute filenames (relative-name invocation failed to resolve files; preserved output). Focused log: build-runs/q/20261023-124802-37048.log.
- Next: run qbuild all once on stable final source, collect terminal result, record gate, commit final report and queue.
- Existing open questions LT12/LT13 retain exactness limits: inherited rounding/control environment (§2.2, §5.7, §5.8.2–4) and NaN encodings (§5.8). No new question; LT1–4 do not block raw owner computations. No algorithm or ABI change.
- No game request outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-lighting-close5-r1-manifest.json.
