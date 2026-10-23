| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netrules-nr17 (observation boundary bytes) | Codex / `re-netrules-nr17` (RE) | in progress | Run documentation checks and complete the NR17 publication report. |

## Handoff

- Goal: close NR17 for glue-netrules-shadow5 with exact GOG observation boundaries and conformance expectations.
- Done: merged NR16 by fast-forward to 6bf9cc16; Git printed a denied packed-refs lock attempt but HEAD advanced.
- Done: exact export bytes independently matched PE reads; published docs/abi/network-rules-observation.md, spec §2.5.6 and vectors 45–49, archived shadow5's NR17 filing.
- Done: question index regenerated; questions check, doccheck check and git diff --check passed.
- Next: commit this milestone then run guard status; finish closure report and merge-queue entry.
- Facts: detached HEAD is intentional; no engine reads/edits. Task-specific instruction to publish in docs/abi is fulfilled by a new document; existing shared ABI tables remain untouched.
- Proposed TSV rows: build-runs/abi-rows-re-netrules-nr17.tsv (docs/abi/vtables.tsv format); their facts are already published in the new document.
- Measured: spans Evaluate 5 to 0x006220D5, ReloadRules 6 to 0x006253D6, cleanup 5 to 0x006232F5; no PE relocations; required base 0x00400000.
- Cleanup: any nonzero flag becomes zero; AL always 1; no table destruction. Observer invalidates even on entry with flag zero.
- Closure source: shadow5 has no Owner closure table; report and NR17 question provide missing-contract checklist. NR17 was only in its unlanded worktree, not our open questions file.
- Evidence script: build-runs/inspect-nr17-pe.py (read-only); no REA session used and no game request outstanding.
- Main checkout: C:/Users/tlori/FreeCity4; manifest: build-runs/run4-re-netrules-nr17-r1-manifest.json in main.
