| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-step-abi4: LV6 native capture ABI proof | Codex / `re-landvalue-step-abi4` (RE spec lane; detached HEAD) | wip: proof/layout proposal b80750e6; full gate pending; no game runs; LV6 installation gap retained; C ABI unchanged | 1. Review/apply pending ABI section and run full train gate. 2. Prove safe installation/startup path. 3. Implement native capture and verify LV2/LV7 traces. |

## Report

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
