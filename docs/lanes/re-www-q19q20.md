| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-www-q19q20 (FTP initialization and HTTP worker/parser contracts) | Codex / `re-www-q19q20` | ready; Q19/Q20 answered; doccheck passed; no ABI change | Coordinator: land specs and use 31 proposed vectors to finish WWW/HTTP implementation credit |

## Report

Q19 and Q20 are fully answered as **DIFFERS**, with no residual split. Changes are
specification prose and tables only. The answered entries are in
[answered/web-network.md](../specs/questions/answered/web-network.md); open entries
were removed and `tools/questions.py index` regenerated `docs/specs/QUESTIONS.md`.
The already-answered Q13 stub that had enclosed Q20 was removed from the open file.

- Q19, commit `682962c9e`: parent §4.13/4.15 and FTP-control §7.1 identify the
  worker's inline initialized-byte store at 008170c6–008170d7. No fallible setup
  dependency belongs to it. Listing helpers 00811f00/00811f90 close listener/data,
  with the older receive-options label corrected. Five WWW19 vectors added.
- Q20, commit `b77852022`: parent §4.14/5.6, HTTP-wire §1.1/1.2 and HTTP-lifetime
  §2.1 specify independent status/version parsing, origin connection even in proxy
  mode, and the stable job layout. +0x14 remains the retained request; a distinct
  response is published at +0x18 before finalization (0081c5d3). Early failures,
  stale responses after redirect, missing releases and callback ownership are
  explicit. Thirteen status, four endpoint and nine resource vectors added.

Evidence: local read-only `re_query.py` queries and per-class export pseudocode;
numeric widths/branches checked against instructions, including wrapping CRT
decimal conversion 009ef7c2–009ef849. Literal bytes and the FTP table were read
from the local executable with a bounded PE reader under this worktree's ignored
`build-runs/`. REA was not used because the session has no binary open. No
decompiled text, assembly listings or game assets were committed.

Validation: native foreground `python tools/doccheck.py check` **PASS**, exit 0,
`build-runs/doccheck-re-www-q19q20.log`; Git whitespace check passed. The earlier
attempt through q.py could not start Bash (CreateFileMapping error 5), so doccheck
did not run in that attempt; its separate log is
`build-runs/q/20261024-222857-28716.log`. Commits succeeded on detached HEAD;
Git's subsequent shared packed-refs maintenance reported a permission error, and
no shared configuration/ref repair was attempted. Cargo/clippy/ctest: n/a.

All **31 vectors are PROPOSED / UNEXECUTED**, not measured game or live-server
results. No game run requested, no shadow verdict, no ABI exports changed and
no ABI rows proposed. No new questions filed. No engine, plugin, capi, ABI table,
coverage-map or coordinator status files were edited.

## Handoff

- Done: Q19/Q20 complete and committed; question index regenerated; doccheck passed.
- Next: coordinator can land this ready RE result and implement/test the specified boundaries.
- Queue: `build-runs/merge-queue/re-www-q19q20.json`, full final HEAD; kind re, abi false, no game runs.
- Source work stopped after the approximately two-thirds budget checkpoint; no residual RE work.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-www-q19q20-r1-manifest.json`.
