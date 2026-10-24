| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-lv8: native capture lifecycle contract | Codex / `re-landvalue-lv8` (reserved detached HEAD) | ready: LV8 answered; doccheck/questions pass; no ABI change | Host must demonstrate dispatch contract before enabling capture; LV2/LV7 remain open. |

## Report

LV8 is answered **DIFFERS** in landvalue §6.1.4: the original provides lifecycle ordering,
not an interceptor admission/drain barrier. The spec now distinguishes native facts from
host acceptance obligations. This is ready spec work, not approval to enable native capture.

- Added a native closure table for early registration, both construction paths, pause/post-init,
  pre-shutdown and retained references. The source lane's current file has a Report, without
  the brief's Owner closure table/Handoff; its LV8 and the spec's explicit obligations were
  used as the closure list.
- Published the host behavior contract: exclude callers before route resolution/mutation,
  drain admitted calls, preserve native identity/order/result, cover startup after completed
  Init, end owner reads before Shutdown, restore only owned routes, and retire resources only
  after all callers and retained callbacks can no longer reach them. No installation API,
  detour instructions or native signatures were invented.
- Distinguished a persistent global forwarder from per-city observation lifetime; keeping
  code resident does not preserve destroyed owner inputs or prove initial publication safe.
- Added prose conformance vectors 26–31. They cover lifecycle-only rejection, both startup
  paths, fetched destinations outside an entry counter, retained notifications, teardown and
  address reuse, validation failure and foreign route ownership. No executable vectors added.
- Archived LV8 verbatim from `glue-landvalue-step6` and regenerated the questions index.
  LV2/LV7 and their existing evidence remain unchanged. No new questions filed.

### Evidence and validation

Used only `tools/re_query.py` and the main checkout's decomp export; no REA call or binary
session was needed. Native instructions verified ordering and lack of a drain operation:
NewCity `0x00454937–0x00454941`, LoadCity `0x00455898–0x004558A2`, close-city
`0x00454A7C–0x00454AC9`, HiddenPause `0x00702200–0x0070221B`, OnTick
`0x00703F05–0x00703F67`, message-server snapshot/unlock/invoke `0x0092EAC9–0x0092EB15`,
land-value Init registration `0x006BE6EB–0x006BE724` and Shutdown `0x006BDF81–0x006BDFA1`.
Both direct Step callers remain DoMessage (`0x006BF116`, `0x006BF160`). No numeric formula changed.

- `python tools/doccheck.py check`: PASS.
- `python tools/questions.py index`, `check`, `find LV8`: PASS; LV8 found only in archive
  in this worktree; index has two open landvalue questions and eight answered, next LV9.
- `git diff --check`: PASS. Review corrected the teardown fixture to release retained
  references before simulating allocation-address reuse.
- Cargo, clippy, CTest and real-game runs: n/a for spec-only changes. No game request is
  outstanding; no shadow-exactness or input-fidelity result is claimed.
- No engine files read or edited; no plugin/capi/tools/coverage/ABI files changed; no ABI
  rows proposed and no C ABI export/version change. Commits are detached as instructed.
- Git emitted a denied `packed-refs.lock` diagnostic during commits, but each commit
  succeeded and actual HEAD was verified; shared refs/config were not changed.

### Landing and next steps

The base did not yet contain step6's LV8 entry. Its text is preserved directly in the archive.
If train178 lands the source question before this lane, remove that now-answered open LV8
entry while merging and regenerate the index; do not resurrect it or discard step6's LV2
runtime evidence. This is the sole cross-lane reconciliation needed.

The adapter lane owns implementation and evidence for the §6.1.4 contract; existing samples
stay `input_capture=pass-end-sample; native_step_capture=0`. Actual per-Step/startup traces
are still needed for LV2/LV7. This answer does not assert an available host implementation.

## Handoff
- Goal: answer LV8 from glue-landvalue-step6; spec text only, no engine reads/edits.
- Done: initial handoff `64ea7840a`; spec/archival milestone `edcdf7d12`.
- Done: native contract, vectors 26–31, question index and doccheck; final report above.
- Next: coordinator lands spec; reconcile open LV8 if train178 introduces it first.
- Then: adapter implementation/evidence under §6.1.4; LV2/LV7 runtime traces remain open.
- No native capture enabled, ABI change, engine read/edit, outstanding game/build request or new question.
