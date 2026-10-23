| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-game-data-registry4: first registry owner contract | Codex / re-game-data-registry4 | ready; bounded first spec; no ABI or game runs | GDR1–GDR4: factory, exceptional inputs, reentry, diagnostics. |

## Report (2026-10-23)

Published [game-data-registry.md](../specs/game-data-registry.md), linked from application §12.
This distinct owner was excluded from the earlier cSC4App rounds. No whole-owner completion credit is claimed.

- Milestones: `833b1ac8` first substantial contract; `89c1cafa` body reconciliation and Lua replay answer.
- Contract: compiled 29 slots; service/QI/reference/lifecycle; node/leaf identity and separate indexes; duplicate/null-key rules; borrowed outputs and Boolean failures; callbacks, flags and contexts; insertion/FIFO/LIFO/postorder distinctions; removal and subscriptions. Fourteen literal vectors cover ordering, identity, references, failures and notifications.
- Inventory: exactly 48 attributed bodies / 5807 bytes: 37 supported bounded behaviors, five partial contracts, six generic/foreign helpers. Three additional nearby bodies explain the 51-body / 6794-byte interval. Shared slot bodies are separate; recovered draw-system/ordinance/grid labels do not establish ownership.
- Identity: version resource 1.1.641.0, original filename SimCity 4.exe; SHA-256 matches export-summary and re/analyze.json (`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`). Widths checked in instructions. Service ID EBE0E860, signed priority -1000000. Base/root-key counters start zero; root owns one key reference after construction.
- Replay: new target receives root/subtree synchronously, reverse-sibling insertion DFS; duplicate false without replay. Legacy Lua Q7 answered as LH3 in Lua §9.1 and archived. Normal broadcast delivers typed targets in registration order; exact equality branch 00473D72 skips ordinary message-target delivery despite message replay working.
- Validation: questions index/check and diff check pass. Ignored `build-runs/check-registry-doc.py` checks 48 unique body addresses/sizes against export, 5807-byte sum, links/replay consistency and all 29 vtable targets directly against PE bytes. Service-face slots 8/9 confirm init/shutdown hooks.
- Admission/tooling: actual manifest and copied host GO0 consumed; normal guard retained. Canonical re_query/export/instructions used first; local decomp supporting only. Known unchanged NoApp not retried; no live target repair, successful pseudocode or target readiness claimed. re_query vtable lookup returned no owner rows, so read-only PE comparison independently verified compiled ABI rows.
- Scope: documentation only; no engine reads/edits, builds/tests, native game/runtime calls, ABI/version edits, generated coverage maps or shared tool edits. No ABI proposal necessary. No game/shadow claims. Commits succeeded despite sandbox-denied packed-refs.lock warnings; detached HEAD retained without ref/config changes.
- Questions: GDR1–GDR4 in [questions/game-data-registry.md](../specs/questions/game-data-registry.md): factory registration, exceptional inputs/allocations, callback reentry, diagnostics. No guessed paths or snapshot guarantee.
- Next: separately assigned bounded follow-up on those questions; supported normal-input vectors do not imply full owner replacement admission.
