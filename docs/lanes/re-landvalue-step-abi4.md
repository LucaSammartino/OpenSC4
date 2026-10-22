| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-step-abi4: LV6 installation/startup proof | Codex / `re-landvalue-step-abi4` (RE spec lane; detached HEAD) | ready: bounded static evidence; LV6 installation gap explicit; docs checks pass; no game runs or C ABI change | Coordinator review/apply/gate ABI proposal separately; reconcile early registration/exclusion and new-city object-only gap before native capture; verify LV2/LV7. |

## Report

Round 2 publishes a completed, bounded evidence report. **A complete safe installation is
not proved; LV6 remains open.** The ready queue is for these specifications and the pending
proposal only. It neither applies `docs/abi/` changes nor authorizes native capture. The
round-1 wip queue discussion preserved below is historical; its ABI/full-gate prerequisite
still applies before an adapter uses the proposed fields.

### Changes and measured results

Proof milestone **5bf0dfc6** adds landvalue §6.1.2 and narrows LV6 in
`docs/specs/questions/landvalue.md`, preserving b80750e6/68c9b4f9's canonical L/M/S bases,
native layout and synchronous forwarding proof.

- Both new and loaded cities synchronously send pre-init `0x26D31EC0` before city Init.
  NewCity's send at `0x00454937` precedes land-value creation. LoadCity's send at
  `0x00455898` follows saved-interface reads. A single pre-init object query cannot cover both.
- City Init creates missing components, then calls land-value Init at `0x0045C088`.
  Native Init prepares progress before registering M at levels 0/10/4/6; initialized is set last.
- Queued post-init `0x26D31EC1` triggers the simulator's HiddenResume. Its name does not
  prove a callback runs before the first native delivery. Ordinary pause still permits idle.
- Synchronous pre-shutdown `0x26D31EC2` at `0x00454ABF` precedes city Shutdown.
  Owner Shutdown removes agents/subscriptions before releasing grids/iterators; QI refs
  preserve allocation lifetime only. Notification dispatch copies AddRef'd targets and
  releases its collection lock before calling them; removal cannot cancel an existing copy.
- Whole instruction prefixes covering a five-byte branch: DoMessage **6 bytes**, continue
  `0x006BF0D6`; Init **5**, continue `0x006BE3B5`; Shutdown **7**, continue `0x006BDF77`.
  Calling bases, stack contract and restoration preconditions are documented. No detour
  encoding, executable patch or hook implementation is supplied.

### Coordinator proposal and remaining work

`build-runs/pending-abi/re-landvalue-step-abi4-layouts.md` remains the exact proposed
`docs/abi/layouts.md` section. This round adds initialized/refcount widths and evidence,
observed allocation size and entry-span observations. No new interface TSV rows are
requested. Coordinator review/application and a **full train gate** remain required before
glue may use these private fields. No direct ABI table, engine, tools or C ABI export changes.

LV6 now identifies three precise missing proofs: observer registration/host availability
before both pre-init sends; exclusion of ticks, notifications and in-flight calls during
installation/restoration; and, for an object-only route, a callback after new-city component
creation but before land-value registration. No such callback is established. These are
implementation prerequisites, not grounds to guess bytes or remove gates.

LV2/LV7 runtime questions and preserved traces remain unchanged. Keep
`input_capture=pass-end-sample; native_step_capture=0`; no input-fidelity, runtime
replacement or restored-map causal credit. Next work is host/ABI reconciliation followed
by real per-Step/state-7 capture and runtime verification, without fitting algorithms or samples.

### Validation and transport

Instruction bytes, bounded decomp reads and export callers/xrefs establish the published
facts. LoadCity's existing decomp failed on a structure field; export instructions supplied
the ordering instead. REA was neither needed nor invoked. `git diff --check` passes;
`python tools/questions.py index` succeeds with no index change. No question moved to
answered; existing LV6 was refined. No cargo/clippy/ctest/full gate or game runs performed.

The quiet Bash wrapper failed E_ACCESSDENIED, so reads used bounded PowerShell/Python.
Git printed a shared packed-refs.lock permission error during commit, but the detached
commit succeeded and its HEAD/clean status were verified; shared refs/config were untouched.

## Preserved round-1 report

Native proof and exact layout proposal committed in **b80750e6**, before call 30. Final review
corrected the slope-pointer store citation to `0x006BE649`. No direct `docs/abi/` edits,
C ABI exports, version changes, engine/tools edits, algorithm/math/golden/sampling changes.

### Published behaviour proof

Landvalue §6.1.1 and §11.3 now distinguish the native land-value interface L from message
target M = L + 4 (serializable = L + 8). QueryInterface grants a reference for each successful
query; native slot 3 requires M, while all progress fields require L. Both native Step calls
are in DoMessage, for SimNewDay (5 ms) and SimIdle (32-bit data3 budget).

The calendar caller invokes slot 3 at `0x007039CB`, then releases its message at `0x007039D4`;
the idle caller invokes slot 3 at `0x00702520`, then releases at `0x00702529`. This proves
the synchronous before/forward-once/after/return boundary and borrowed message lifetime.
The spec records iterator inclusion/exclusion, state-3 fallback reset, developer cursor/end
semantics, pre-call copying of each developer's current int16 map, and observer-owned copies.

### Exact public ABI proposal (not yet applied)

**File:** `build-runs/pending-abi/re-landvalue-step-abi4-layouts.md` (force-tracked so the proposal
survives delivery). Append its proposed section to `docs/abi/layouts.md` after review.
No interfaces.tsv rows are proposed: existing slot rows cover the required calls.

Verified: state +0x38 is 4 bytes; iterator pointers +0x3C/+0x40 are 4 bytes; ignoreZoning
+0x4C is 1 byte; cursor +0x70 and header +0x8C are 4 bytes, all relative to L.
Iterator allocation is 0x20 bytes, with 1-byte state and seven signed int32 fields.
Known developer-node links/key are 4 bytes; weight +0x14 is float32, verified by the callback's
float load at `0x006BD671`. Node colour/padding/allocation size are intentionally unspecified.
The proposal identifies native vtables, IIDs, widths, evidence addresses and version-validation
requirements. The measured working-copy executable SHA256 is recorded as an observation,
not a pristine-build certification or an implemented detector.

### Remaining questions and validation

**LV6 remains open** in `docs/specs/questions/landvalue.md`: QI installs no interceptor.
No path is proved that preserves existing agent identity/order, shutdown removal and
notification lifetime while installing early enough to capture loaded previous maps.
Native Init registers M at levels 0/10/4/6; AddAgent references its supplied target.
This evidence does not approve proxy re-registration or vtable mutation. No question moved
to answered; LV2/LV7 runtime gaps and `native_step_capture=0` remain unchanged.

Validation: export bytes/callers and bounded decomp reads only; no REA needed or errors.
`git diff --check` passed; question index regenerated with the existing question count retained.
No cargo/clippy/ctest/full gate or game runs performed; no runtime fidelity credit claimed.
**Coordinator must review/apply the ABI proposal and run a full train gate.** Queue state is
`wip` to prevent an ordinary docs-only merge from bypassing that requirement. Once ABI records
are gated, installation proof and LV2/LV7 runtime capture remain separate prerequisites.


### Coordinator publication review (train78)

Applied the bounded native layout proposal to docs/abi/layouts.md after independent checks against the native executable of QI +4/+8 views, initialized/refcount widths, iterator allocation/state, message-target adjustment and developer cursor/header. Entry-prefix lengths were independently decoded as6/5/7 bytes for DoMessage/Init/Shutdown. Full native gate required before landing. These read-only observations grant no installation or construction permission; LV2/LV6/LV7 remain open and native_step_capture stays0. Original report queue wording above predates final ready publication.
