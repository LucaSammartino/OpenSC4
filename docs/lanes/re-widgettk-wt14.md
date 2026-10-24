| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgettk-wt14 (private buffer and graphics contracts) | `Codex` / `re-widgettk-wt14` (RE lane) | ready; WT14 answered; 14 conformance vectors; doccheck passed; ABI unchanged; game n/a | 1. Land queued tip. 2. core-uiwin-close13 can implement WT14 from sections 14.5.1–14.6. |

## Report

Closed WT14 in `docs/specs/widget-toolkit.md` sections 12 and 14.5.1–14.6, moved its original entry to
`docs/specs/questions/answered/widget-toolkit.md` with the answer marker, and regenerated `docs/specs/QUESTIONS.md`.
No new questions were filed. Other open questions remain outside this task, including WT13's derived destructor.

The contract distinguishes destructive draw-service replacement from fallback reuse of the owned buffer.
It specifies the signed both-nonpositive dimension rejection, by-value format `(1,0)`, output mutation and
failure/reference effects, same-size bypass and the common tail after allocation failure. Fourteen recording-host
vectors cover these effects, fade gating and drawing-cache ownership. The graphics table names the previously
opaque context, buffer, graphics-system and colour-manager operations and points to their separate owner specs.

Corrected the destroy path: it tests existing fade/paint bits rather than setting them. Buffer slot39 is the
plain buffer's initialized byte but constant true on the draw-service wrapper; wrapper Shutdown is a true no-op,
and wrapper Init always fails. Presentation interface slots4–6 return borrowed contexts and manage target/access
state rather than acquiring an owning context reference. No recovery is invented for native service allocation
exhaustion that continues through a null pointer.

Evidence: export queried before bounded decomp reads; all dimensions, descriptor dwords and allocation/failure
decisions checked against instruction bytes. Main addresses: slot150 `0099D0ED`, PrivateBuffer `0099EA70`,
SetBufferToDrawTo `0099C6F8`, draw-system slot5 `0082B980`, descriptor helpers `0044A490`/`00449500`, plain
Shutdown/readiness `00825CE0`/`008264B0`, wrapper constants `00648F00`/`00735290`, wrapper context access
`0082AF20`/`0082AF50`/`0082ABE0`. Selected vtable pointers were checked read-only in the working EXE using
`build-runs/read-widget-tables.py`. No REA calls: no binary is open in that session; export/decomp sufficed.

Validation: final `python tools/doccheck.py check` **passed** (`build-runs/doccheck-wt14-final.log`);
`git diff --check` passed. Questions index regenerated successfully. Cargo, clippy, native tests and game/shadow
runs are n/a for this spec-only lane; no game requests are outstanding. No engine, plugin, capi, tools coverage
map or ABI files changed; no ABI rows proposed. The first commit succeeded despite a subsequent Git maintenance
attempt reporting a denied shared packed-refs lock; no maintenance or shared-ref repair was attempted.

## Handoff
- Goal: close WT14 for core-uiwin-close13; spec text only.
- Done: `7963eb6b5` added WT14 behavior, ownership corrections and 14 vectors, moved WT14 to answered archive, regenerated index.
- Done: final review added wrapper-specific readiness and presentation-access details; final doccheck and whitespace checks passed.
- Next: coordinator lands queued full HEAD; engine lane consumes sections 14.5.1–14.6. No further RE step required for WT14.
- Evidence: slot150 0099D0ED; draw-service slot5 0082B980; SetBufferToDrawTo 0099C6F8; byte checks establish signed OR dimension gate and format (1,0).
- Facts: actual paths are docs/specs/widget-toolkit.md and docs/specs/questions/widget-toolkit.md (brief's doubled paths are typos).
- No ABI changes or game requests; use export/decomp only, no REA binary available.
