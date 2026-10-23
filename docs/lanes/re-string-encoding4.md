| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-string-encoding4 (dedicated converter spec) | Codex / `re-string-encoding4` (RE spec lane, detached HEAD) | ready: docs only; six entry ranges audited, 51 PROPOSED/UNEXECUTED vectors; original bytes 30/30; doc/link/literal/diff checks PASS; no ABI, questions or game runs | Coordinator: publish detached tip to reserved lane branch and land normal docs queue. |

## Report (2026-10-23)

Added [string-encoding.md](../specs/string-encoding.md) and only a narrow canonical
link in strings-language §9. Commits: `a8fdc428` substantive counted dispatch and
first literals before call 25; `3189ce30` loops, bridge, wrappers and complete
proposed ledger; `cc19dcfa` canonical link and evidence-check milestone.

### Original family/budget audit

This is the first dedicated whole-converter research round, original family round
1 of 3, with this admission limited to **one** round, 100 calls since commit and
150,000 context tokens. No future round, reset or additional admission is requested.
It is not a claim that these instructions were previously untouched.

- re-strings4's original round-2 report explicitly says ConvertStringEncoding not
  started; its rounds covered Utility/Manager. Original final storage task
  `build-runs/run4-re-string-storage-final4-spec.json` expressly excludes
  “ConvertStringEncoding as a separate owner.” Parent 2 + final child 1 remain
  exhausted for the old storage/language family, without a name-based reset.
- re-answers10 FS16 already publishes a **partial** exact `00910E18` UTF8→UTF16
  decoder branch within MakeLower. graphics-formats §3.11 is unchanged and cited;
  low16/no-surrogate, invalid/truncated, surrogate/unusual lead behavior and the
  prior U+1F600→F600 / invalid Latin1 Café truncation witnesses remain prior facts.
  Ledger rows exercising that branch compose it with the new contract; they are
  not new proof of untouched full-converter coverage.
- Main-checkout coordinator evidence `coordinator-string-encoding-first-eligibility.md`
  (completed review 08:08 BST) and `coordinator-string-encoding-first-host-proof.json`
  (08:05, FULL strings and language / kind RE check GO exit 0) retain that overlap
  and the separate-owner exclusion. The original brief peer is that predispatch
  review; no additional agent or external message was used.
- Current manifest `run4-re-string-encoding4-r1-manifest.json` matches this exact
  worktree, lane, kind RE and round 1. Runtime row role RE/touches strings and
  language/rounds 1, no RE pause; codexguard within budget at each milestone.
  No same-whole-converter family or remaining-budget contradiction was found.

### New owned contract

Selector resolution order; whole-count ASCII shortcut before equality; raw-copy,
selector-0 widening and low-byte narrowing; byte/word units and no count sentinel;
embedded zero and no added terminator; branch-specific full-width count returns;
UTF16 encoder's read-before-capacity truncation; typed direct Win32 calls and their
zero-capacity sizing paths; unsupported unequal legacy pairs; private EDI-count
two-stage UTF16 bridge and its count comparisons; four string wrappers' allocation,
getter/assignment/free order and AL/count return widths.

Notable consequences: UTF16→UTF8 may return N despite not emitting the last word;
bridge failure can reset the output count without undoing already-written bytes;
wrapper failure still assigns a reported prefix; target-UTF16 string wrapper passes
the **word count unchanged to byte assignment**. OS calls remain individual raw
primitives with explicit counts, replies and bytes, never whole-converter oracles.

Six metadata ranges (provisional total 2,159 bytes) are distinct owned roles, not
aliases: core, private bridge, four wrappers. Per-entry Supported/Partial-prior/
Foreign/Unknown ledger records scope; no blanket native/verified/full-byte credit.
Foreign parse gap `00910BD3–00910E17`, GetUTF16Character from `009115F0`, allocator,
cleanup, string methods, CRT/Win32 and system-codepage primitives are excluded from
new body credit. No Thai/font/TextEdit, cRZString storage, startup or advice owner
was reopened. Invalid pointers, allocation failure, raw alias/overlap, overflow and
nonreturning domains remain Unsupported; no safe rollback is invented.

### Evidence and checks

- Primary matching Windows export: func, callers, callees and paged asm --bytes;
  only targeted decomp navigation. No REA invocation or session opening was needed.
- Ignored read-only helper `build-runs/check-string-encoding-docs.py` maps original
  `bin/SimCity4.exe` PE sections: **30/30** cited instruction-byte anchors match.
- **51** sequential unique SE01–SE51 literal/reply/order vectors, all explicitly
  **PROPOSED / UNEXECUTED**. Six literal-presence checks and eight independent
  operand-width/threshold arithmetic assertions pass. No modern decoder oracle,
  portable implementation result, native execution, fixture execution or shadow.
- Relative link files and heading fragments PASS; git diff --check PASS.
  Final scope is exactly this report, the new spec and strings-language §9 link.
  graphics-formats, all questions, tools, ABI tables and shared indexes unchanged.
- Cargo/clippy/ctest: n/a (docs-only RE lane). Game runs: none; ABI exports: unchanged.
  New/answered questions: none; existing SL1 and other owner questions unchanged,
  so no question-index generation was needed or performed.

### Original errors and corrections

q.py's Bash backend returned E_ACCESSDENIED; used capped PowerShell and local
Python thereafter, without rerunning host preflight. Guessed wildcard/string-storage
navigation searches returned no matches; actual supplied reports/tasks and explicit
export/decomp offsets supplied the evidence. Detached commits print a nonfatal
packed-refs.lock permission denial but commit successfully; no ref/config/maintenance
workaround was attempted. Intermediate SE28 wording was corrected before its ledger
milestone commit; final expected raw bytes are unambiguously E9 CC CC CC.

### Next steps

Coordinator publishes the clean detached tip to the reserved branch, then consumes
the own-worktree normal docs queue (`abi=false`, `game_runs=[]`). No native test,
implementation, extra research round, shared generation or ABI row is requested.
