| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-proplot-pl23 (raw profile and foundation fields) | Codex / detached `re-proplot-pl23` | ready: PL23 answered; doccheck and questions check PASS; four prose vectors | Land docs; retain raw mismatches and limited comparison credit. |

## Report

PL23 is answered **MATCHES**: there is no deterministic exemplar-derived native value for ordinary prop
profile bytes +0x15..+0x17 or an appended foundation object's +0x24 ID. The original leaves these stack
bytes unwritten and subsequently copies them into the catalogue/array. FreeCity4's explicit zero choices
remain valid choices, with no promise of native raw equality. No implementation change can be justified by
the observed unspecified memory alone.

- `prop-lotconfig.md` §A3.2 and §B3.3 now state the raw-comparison contract. Test 1 distinguishes native
  wealth-byte defaults from replacement padding defaults; test 16 adds PL23-P/Z/F/D prose vectors for
  nonzero wealth padding, absent-wealth padding, appended foundation ID, and an additional real mismatch.
- `lot-configuration.md` §3.7 states the same foundation rule at its owning section.
- PL23 moved to `docs/specs/questions/answered/prop-lotconfig.md`; question index regenerated. No new
  question filed; other open questions untouched.
- Evidence: export instruction bytes show the one-byte profile default/read at `0x004A3E52`/`0x004A3E5A`
  and seven-dword copy at `0x004A3FDE`–`0x004A3FE7`. Foundation construction at
  `0x0047A59E`–`0x0047A609` skips +0x24; object copy `0x0047892B`–`0x00478931` preserves it. Export and
  targeted decompilation were sufficient; no REA request or error occurred.
- Read the current glue-dormant-shadows5 worktree report first for closure requirements. Its current file
  has no Owner closure heading or `open:` rows; its owner mapping and retained-PL23 section require raw
  differences to remain visible. Answering PL23 does **not** promote either owner to raw-exact status.
- Supplied evidence, not new measurements: `vpc4-r2-native-20261022a` reported 1544/1544 profile raw
  differences and 1908/1909 configuration raw differences. The dormant lane reports propmgr 219/220 and
  lotcfg 292/293 clean shadow lines. Six final-record first differences do not prove all remaining array
  fields equal. A full field diagnostic remains necessary before claiming that narrower scope.
- Validation: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS; PL23 lookup
  resolves only to the answered archive. Cargo/clippy/CTest not applicable to documentation-only edits.
  No game runs requested or executed, no native byte copying or filtering, no ABI rows/exports changed.
- Work milestone: `c2691ae80`. The commit completed despite Git printing a packed-refs lock permission
  warning; HEAD and clean status were verified. No refs/config repair or branch switch attempted.

## Handoff

- Goal complete: PL23 rule, archive and prose conformance vectors committed; documentation checks pass.
- Next: coordinator lands this documentation and closes the RE dependency. Retain the reported raw
  mismatches; whole-owner exact credit is still unsupported. No rerun of unchanged algorithms is required.
- Queue: `build-runs/merge-queue/re-proplot-pl23.json`, kind re, state ready, actual detached HEAD.
- No outstanding game requests, proposed ABI rows or unresolved question within this task.
