| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dbpf-ex57: transient DBPF observation | Codex / re-dbpf-ex57 (detached HEAD) | ready; doccheck and questions check pass; EX57 answered; ABI unchanged | Coordinator: land with glue-dbpf-ex56apply11; preserve transient coverage limits. |

## Report

Answered EX57 in [data-formats §8.3.1](../specs/data-formats.md#831-transient-segments-observation-boundaries-ex57-compat).
Copied the exact question ID/text from glue-dbpf-ex56apply11 before answering, moved it to
[the archive](../specs/questions/answered/data-formats.md), and regenerated the question index.
The engine lane's report at that ref has no Owner closure table; its remaining-scope/open-EX57
paragraphs explicitly exclude transient files, and those exclusions remain justified.

The spec adds typed persistence/streaming interface distinctions, message field types, borrowed
ownership versus acquired QI references, synchronous load read intervals, save-phase limitations,
and typed original helper boundaries. Nine prose conformance vectors EX57-1–EX57-9 cover these
contracts in §10.9. No implementation, ABI rows, C exports or ABI version changes.

Measured static findings:

- Ordinary load has three segment-bearing notifications while the package is readable.
- PreSave, unnamed 0x26C63346 and Save carry the temporary persistence segment; PostSave does
  not set void1. PostSave precedes root-stream teardown and class-object serialization.
- Regional and obliteration local lifetimes send none of those ordinary lifecycle notifications.
  No common public all-open-files observation callback is supplied by these paths.
- OpenSaveGame checks unsigned version 62..65 when requested. CloseSaveGame consumes its
  reference only if IsOpen succeeds; regional successful-open teardown has the same conditional
  skip. The obliteration terrain writer returns true even on Init/Open failure.

Evidence: read-only main-checkout re_query.py function, vtable and instruction queries (with bytes),
targeted decomp sections, and original binary vtable pointer validation. cSC4App pseudocode was
unavailable in the export, so its instruction pages were used directly; no REA call was attempted,
as instructed. No engine/, plugin, capi, docs/abi or coverage-map file was read or edited.

Validation: `python tools/doccheck.py check` passed; `python tools/questions.py check` passed;
`git diff --check` passed. Foreground gate output: `build-runs/doccheck-ex57.log` and
`build-runs/questions-ex57.log`. Initial q.py attempts failed to start Bash with Win32 error 5
before executing either gate; the successful Python-runner results are separate. Cargo, clippy,
CTest and game runs: n/a (spec-only). There is no new shadow exactness or transient coverage claim.

Commits: dc29f7142 initializes/copies EX57; aa43d9311 commits the answer, vectors and index.
Ordinary detached commits succeeded despite Git's packed-refs.lock diagnostic; shared refs were
not changed. The supervisor publishes the final detached tip to the reserved branch.

## Handoff

- Complete: EX57 answered DIFFERS, documentation gates pass; no new questions filed.
- Next: merge train lands this tip with glue-dbpf-ex56apply11. If the other branch reintroduces
  the open EX57 entry, retain the archived answer and remove the open duplicate, then regenerate index.
- Engine follow-up may observe ordinary save phases, but complete-save comparison uses the
  registered backing file after successful save. Full transient coverage needs extra instrumentation.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-dbpf-ex57-r1-manifest.json.
