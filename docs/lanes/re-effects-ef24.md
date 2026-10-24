| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef24: native observation ABI and sequence extent | Codex / `re-effects-ef24` (RE, detached HEAD) | ready: EF23 fcc39606a, EF22 61b57bc20; doccheck PASS; 19/19 byte guards and instruction spans; 17 proposed unexecuted vectors; no ABI exports changed; no game run | (1) Host implements validated attachment and proves writer/unwind conditions or refuses; (2) measure natural history and payload before claiming owner coverage. |

## Report

Answered **EF22 and EF23** from glue-effects-whole9 in effects.md; moved both
original entries to questions/answered/effects.md and regenerated
docs/specs/QUESTIONS.md. No open question was added. No engine, plugin, C API,
tools/coverage-map.tsv, ABI table, game asset or decompiled output was changed
or committed. No proposed ABI rows are needed; C ABI exports are unchanged.

### Changes and evidence

- **fcc39606a / EF23:** corrected sequence extent from 0x18 to **0x1C**;
  vector at +0x0C and full retained u32 flags at +0x18 are payload, including
  anonymous descriptions and empty vectors. Constructor zero store
  005BDF3B, allocation 0059E69E, stream Read mask 7 at 005BE2D2, unmasked
  Write at 005BD703. Memory capture does not apply the stream mask.
  Five proposed conformance vectors cover bounds, flags and stream round trip.
- **61b57bc20 / EF22:** sections 18.1 and 18.8.6–18.8.7 give the inspected
  executable fingerprint, sixteen complete entry spans and three indirect
  call-site spans, expected bytes, continuations and receiver conventions.
  Publication's relative tail jump must retain target 0058D0E0 and R+4.
  The message table's slot 3 contains **00594120 directly**, taking M+8;
  it is not a separate subtract-8 thunk. Resource collection/persist thunk
  adjustments are -4/-0x11C; the primary reference wrappers add 0x11C.
  Twelve proposed conformance vectors cover guards, forwarding, lifetime,
  exceptions, removal and writer refusal.
- Native interfaces provide **no complete observer-registration mechanism,
  universal writer lock or exception-safe detour installer**. Vtable-only
  substitution misses direct calls and early resource Read. Installation,
  removal, native unwinding and all-writer exclusion remain explicit host
  obligations. The spec requires refusal when they cannot be established;
  a matching calendar callback or zero observed depth cannot prove exclusion.

Evidence came from main-checkout tools/re_query.py first, then bounded decomp
reads and instruction-byte verification against bin/SimCity4.exe. REA was not
called: the brief states that no binary is open. Local helpers and evidence
logs remain only in this worktree's ignored build-runs/.

### Validation and limits

- **doccheck check: PASS**, final log build-runs/ef24-doccheck-final.log.
- **19/19 expected-byte guards PASS**, all nineteen end on complete exported
  instruction boundaries, **zero exported direct branch/call targets inside**
  the displaced intervals. Log: build-runs/ef24-byte-guards-final.log;
  local helper: build-runs/validate-guards.py. This static check excludes
  neither computed jumps nor foreign runtime patches, as the spec states.
- git diff --check passed. Question-index generation completed after both
  closures. Cargo, Clippy and CTest are not applicable to these text changes.
- **17 conformance vectors are proposed and unexecuted**, not executable
  fixtures or native goldens. No native interceptor, game run, stream capture,
  payload-equivalence or whole-owner verification is claimed. The prior lane's
  names-only 73/73 result is unchanged; this lane adds no measured coverage.

Execution notes: q.py failed to start Bash (CreateFileMapping error 5); direct
PowerShell foreground commands completed the index and gate. An initial git
add used a nonexistent question-index path; the corrected commit stages the
actual docs/specs/QUESTIONS.md. Git emitted a shared packed-refs lock denial
but both content commits succeeded on detached HEAD; no shared-ref repair was
attempted. No gate or game request remains running.

## Handoff

- Goal complete: EF22/EF23 spec answers, archive, index and doccheck.
- Content commits: fcc39606a (EF23), 61b57bc20 (EF22); initial handoff bfdcd9f3a.
- Next: coordinator publishes detached tip to reserved lane and lands ready RE queue.
- Implementer starts at effects.md §§18.8.4,18.8.6–18.8.7.
- No native lock/observer API was found; missing host exclusion or unwind proof means refusal.
- No open questions, game requests, ABI rows or gate operations outstanding.
