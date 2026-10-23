| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-ww5: web and network contracts | Codex, `re-www-ww5` (reserved detached HEAD) | ready: bounded partial spec; doccheck passed | Implement qualified contracts; settle Q3/Q11–Q13 before whole-owner claims |

## Report

Published a **bounded partial extension**, not completion of the requested approximately 72 KB
of owners. [web-network.md](../specs/web-network.md) now adds:

- Section 3.6–3.7: cookie constructor defaults, lifecycle, enumeration, persistence selection,
  unsigned expiry filtering, contiguous entry reads and the native 20-slot inventory.
- Section 5: HTTP native dispatch associations and named-method inventory, links to existing
  admission/lifetime work, synchronous message payload ownership, exact progress arithmetic and
  threshold, and the runtime date conversion boundary.
- Section 6: cache's 28-slot inventory, defaults, lifecycle/path handling, expiration, RAM/file
  access, byte admission, failure effects and the persistence writer boundary.
- Section 7: links to existing socket contracts and their per-body ledger, plus command-generator
  interface identity, defaults, lifecycle, idle message dispatch, polling and accepted-command IO.
- 36 independent literal vectors in tables: CK01–06, HTTP01–09, NFC01–12, CMD01–09.
  All are **PROPOSED / UNEXECUTED**, not game measurements or implementation-generated outputs.
  No files in engine/conformance were created because this lane may not touch engine/.

Compatibility distinctions include signed read expiry versus unsigned cache sweeps, deletion of
an old entry before rejecting invalid replacement data, null file-output suppressing materialization,
cookie read clearing memory before checking the file, omission of session-cookie sentinel 2 from
ordinary-time persistence, unsigned wrapped progress deltas, and strict elapsed >250 command polling.
Instruction evidence also retains the accepted-command branch that pseudocode incorrectly removes.

Evidence: export-first `tools/re_query.py` function/search/vtable queries and paged `asm --bytes`;
bounded reads of per-class pseudocode. No REA call was needed or attempted; no session error or
fallback occurred. Addresses and the relevant width/branch citations are in the spec. Existing
WWW/FTP and native socket/lookup contracts were linked instead of duplicated. Parked WWW Q7/Q9
were not reopened and no coverage-map changes were made.

Questions in [questions/web-network.md](../specs/questions/web-network.md): new Q11 (cache
source-file admission, filename/loader/holder details), Q12 (command wire grammar/execution),
Q13 (HTTP wire/finalization/date parser). Q3 remains open with partial persistence evidence added;
no question was moved to answered. `python tools/questions.py index` completed successfully and
did not change the generated index because this spec's index entry was already present.

Validation: `python tools/doccheck.py check` **passed**; `git diff --check` passed. Cargo, clippy
and ctest: n/a (spec-only changes). Game runs/shadow comparisons: n/a; no socket or live request
was opened. C ABI exports unchanged; no proposed ABI-row file. No engine, plugin, capi, tools,
README, HANDOFF or ABI-table edits. Git emitted a denied packed-refs.lock maintenance message,
but both commits completed and the working tree was clean afterward; no refs/config were changed.

Milestones: `9ad8d349` cache contract; `a04b3984` cookie/HTTP/command contracts and questions.
RE/spec investigation stopped at measured context 98,867 / 150,000, below the two-thirds limit;
subsequent work is reporting, validation and queue publication only.

## Handoff

- Landing state: ready for review/merge as partial documentation; not whole-owner completion.
- Next implementation work: use only the qualified contracts and proposed vectors; unsupported domains stay explicit.
- Next RE work: Q3 and Q11–Q13; preserve parked WWW Q7/Q9 limits and native transport's unsupported ledger.
- No pending game requests, ABI proposals or external dependencies.
- Supervisor publishes detached commits to reserved branch `re-www-ww5`; lane must not move refs.
