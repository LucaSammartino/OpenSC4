| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-missing-plugin-reporter4 (original reporter contract) | Codex / `re-missing-plugin-reporter4` (RE spec lane) | in progress | (1) Verify source identity and reporter instructions; (2) publish bounded contracts and proposed traces. |

## Handoff

- Goal: first dedicated original `cSC4MissingPluginDataReporter` spec, two own bodies; foreign dialog remains primitive.
- Done: `f26773d7` first original-supported message/set-count/popup-AL/return/allocation-null contracts; substantive contract written before call 20. Full effect/lifetime contract, 15 proposed/unexecuted trace witnesses, three bounded questions and narrow UI inventory link now written.
- Source: working PE FileVersion `1.1.641.0`, SHA256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, matches Windows export summary.
- Measured: export instructions equal mapped PE bytes (post-init 532, message 33); intrinsic adapters identify full-word dedup, AL conversion gate and shared list cleanup. Manager query is occupant-manager slot 38, not a resource query (initial draft corrected).
- Next: commit this contract milestone, review docs-only diff/link scope and finalize report/own exact-tip publication.
- Questions: MPR1–MPR3 in `docs/specs/questions/missing-plugin-reporter.md`; no answered entries moved. Task bans shared QUESTIONS edits, so do not regenerate that shared index.
- Constraints: docs only, no engine, ABI edits, builds, native runs, REA prerequisite, subagents or shared state changes.
- Tool facts: q.py Bash wrapper failed with Bash CreateInstance E_ACCESSDENIED; qgrep default encoding failed on Unicode. Use capped PowerShell reads and Python `-X utf8`.
- Other tool facts: optional pefile import unavailable; corrected analytic helper uses stdlib PE parsing successfully. Initial commit printed packed-refs.lock permission error but created detached commit successfully; do not modify shared refs/config.
- Dependency `core-local-grid-helper4` is operational merged-claim provenance only; no implementation reading.
