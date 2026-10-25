| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-replace16 (factory ownership and guarded native shadow) | `Codex` / `glue-effects-replace16` (engine lane) | Ready measured partial: cargo 7501/0, clippy clean, CTest 85/85; native names 73/73 to 2032; replacement remains guarded and is NOT ready for credit | Do not append effects to the house replace list; complete the native adapter in effects section 18 |

Code: manager factories now start at refcount zero; QueryInterface returns the documented message-target sub-object at +8. The native shadow is selected from actual registration rather than the enabled subsystem key.
Registration stays behind `effects_replace = 1`, an unsafe diagnostic opt-in. The startup log now explains the native-layout limitation instead of implying that replacement has been verified. Shadows remain default-off.
Four plugin CTests cover forced registration and creation, disable precedence, defaults, and guarded native-shadow selection; they also check message-target offset, dispatch, and balanced references.
Final gate: `qbuild all` PASS; workspace 7501 passed / 0 failed; Clippy 0 diagnostics; native build 49/49 edges; CTest 85/85. Native request: `qbuild-20261025-012830-build-ctest`.
Exact measured line: `glue-effects16-guarded-names-20261025-r2 73/73`.
That house-profile turbo run reached 2032-01-01 clean: ref_warnings=0, foreign_exceptions=0, panics=0, other warnings=0; 108.844 real seconds, DLL SHA256 prefix `14af8801`.
Record: [final verdict](../../build-runs/game-results/glue-effects16-guarded-names-20261025-r2.verdict); [full log](../../build-runs/game-results/glue-effects16-guarded-names-20261025-r2.log).
Scope: native name membership only; 1154 entries, 3465 queries per sample, refusal=0 and mismatches=0 on every load/month line. Payload/history remain unavailable; this does not verify replacement, rendering, collections, or parser command execution.
Replacement trials `glue-effects16-iso-20261025-r1` and `glue-effects16-iso-20261025-r2` both registered the class and entered Init, then faulted before a city date (foreign AV writing 0x4D; no ref warning or Rust panic).
`glue-effects16-house-20261025-r1` likewise ended corrupt before a date (foreign AV at 0x18). These disproved the premise that the replacement already runs clean; their logs/verdicts remain under `build-runs/game-results/`.
`glue-effects16-guarded-names-20261025-r1` was clean and 73/73 but contained an unsupported request-only `pacing` key; r2 removed that key and is the final evidence.
Build evidence: the first factory test caught a leaked constructor reference. A header-only fix left its object stale; initialization is now explicit in the constructor source. Both failed native logs are preserved in `build-runs/fc4verify-ctest-failed-cb162c09c.log` and `build-runs/fc4verify-ctest-stale-object-1fc0745dc.log`.
Open: the full native layout/collection adapter is still missing; section 18.5 leaves internal-base contracts unresolved. A SPEC-GAP comment marks the guard. No native signatures were guessed.
Questions filed: none, per revised lane rules. No tools, coverage maps, shared shadow profile, ABI exports, or ABI version were changed.
Next: implement the remaining native adapter contracts and repeat genuinely registered isolated/house replacement runs to 2032 before granting replacement credit.
