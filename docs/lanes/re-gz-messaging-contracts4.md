| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gz-messaging-contracts4 (GZ1–GZ7 native contracts) | `re-gz-messaging-contracts4` / `re-gz-messaging-contracts4` (RE docs, detached HEAD) | ready; GZ1–GZ7 answered; 14 independent prose vectors; questions check and diff check PASS; cargo/clippy/ctest/game n/a; no ABI exports | Coordinator: apply native lifetime/order corrections to partial messaging core; preserve defined cancellation witness and separately choose handling for invalid native iterators. |

## Report

Resolved the seven filed contracts in `docs/specs/framework-gz-services.md`, archived the original
entries with dated MATCHES/DIFFERS answers, and regenerated `docs/specs/QUESTIONS.md`. Added an exact
entry/helper-body inventory (§5.2), callback pointer adjustments, numeric widths, and 14 independent
prose vectors (§5.1). No engine, tools, maps, ABI tables, game files or external files were changed
or committed; no implementation/test outputs were used as evidence.

Native results and reconciliation against the choices recorded in the questions:

- GZ1 MATCHES: slot 12 reads the shared unsigned service reference count at server +0xC, initialized
  to zero; +4/+4 and -8 forwarding paths now explicit. It is unrelated to posted/taken counts.
- GZ2 newly specified: unsigned microseconds divided by 1,000, special infinite value retained;
  queue waits are one second. Mode-0 Signal calls PulseEvent, so missed pulses leave no wake token.
  Wait returns 0/1/2; blocking gets ignore failures/timeouts and retry. Destruction closes the
  embedded event without joining/waking waiters; failed handle creation has no queue recovery.
- GZ3 DIFFERS from dead-path premise: message GetType and target AddRef/Release run at positive
  registry depth. Controlled callbacks reach pending work; ordinary DoMessage runs at depth zero.
  No measured in-game occurrence or exhaustive census of native re-entrant targets is claimed.
- GZ4 positive native callers: intro-video screen self-posts three record types through server 1;
  two unnamed server-2 bodies are verified by actual service acquisition and +0x1C dispatch.
  This proves observable directed order. No name is guessed for unnamed bodies and no concrete
  slot-8 game caller was established; cancellation contract comes directly from native bodies.
- GZ5 DIFFERS: server 1 copies a 16-byte message with a raw target pointer, making a 20-byte entry;
  neither target nor message is reference-counted on append/cancel/delivery.
- GZ6(a) DIFFERS: server-2 snapshot entries hold references until all callbacks finish, then release
  forward. (b) MATCHES: removed node unlinked before Release at depth 1. (c) DIFFERS in timing:
  directed deque holds drop before callback; local pair lives across iterations. Replacement is
  target then message; final message-then-target Release occurs after unlocking.
- GZ7 MATCHES: all five equality operations call queued.equals(argument), testing only AL.

Further contract consequences within these questions: signed dword directed quota is shared by
nested delivery; cancel/post can admit new entries during the same pass. Both cancellation bodies
advance before erase and ignore erase's return. Single-block [B,C,A,A], cancel A, deterministically
leaves [B,C,A]. Matching the last/sole entry can advance beyond the new end; no deterministic
post-invalid-iterator result is invented. Server 1 pending adds use manual holds, pending removals
are raw; direct apply transfers the first add hold and leaks a duplicate add hold on raw bulk clear.

Validation: `python tools/questions.py index`, `python tools/questions.py check` PASS;
`git diff --check` PASS. Cargo, clippy, ctest, game runs and shadow comparisons n/a for this docs-only
lane. No C ABI export changes; no ABI proposal file. No aggregate owner/byte coverage credit.

Evidence: read-only `re_query.py` function/asm-with-bytes/callers/vtable queries against the primary
Windows export, bounded decomp reads, native PE pointer-table verification, and acquisition-seeded
instruction caller search. No REA call was needed. Bash q.py failed with WSL CreateInstance
E_ACCESSDENIED; bounded native PowerShell commands were used. Git emits a packed-refs.lock permission
diagnostic under the restricted detached checkout, but commits succeed and refs/config were not edited.
Ignored local helpers remain in this worktree's `build-runs/` only.

Milestones: `a62435e4` initial numeric/signal/equality/snapshot/value contracts; `251ef9b6` registry,
directed lifetime/order/callers; `446a59b1` vectors, method inventory and seven archived answers.
Coordinator next: implement the native mismatches and independent vectors in the partial core;
decide safe handling for the explicitly invalid native iterator cases separately from defined parity.
