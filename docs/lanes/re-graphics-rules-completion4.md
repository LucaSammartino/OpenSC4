| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-graphics-rules-completion4: graphics rules contracts | Codex / `re-graphics-rules-completion4` (RE spec) | ready: GR1–GR3, named-manager inventory, 10 prose vector groups; native group defaults verified in 2359bd56; diff/questions passed; ABI unchanged; game/shadow n/a | Implement from §§1–12 and independent vectors; no remaining RE action in this bounded task. |

## Report — 2026-10-22

The remaining group-default contract is verified. `docs/specs/graphics-rules.md` §4.1 now cites the
native constructor and its option-group caller; §12 inventories that supporting value constructor,
and `docs/specs/graphics-rules-vectors.md` V5 carries the same evidence. The prior default-zero rule
matches the native program and needed no behavioral correction.

At `007D0410–007D041F`, the 16-byte body clears dwords at offsets 0, 4, 8 and 12 and returns the
record address. These represent recommendation zero and an empty options list in this use.
`007E5542–007E5552` constructs that value and passes it with index+1 to group resize `007D0FD0`.
The export's `cSC4ResponseCurve` label is shared across callers; the option-group call site establishes
the interpretation here. Export instruction bytes were checked against the original executable at
`007D0410` and `007E5549` using the existing read-only helper. No decompiled text or instruction
listing has been copied into the spec.

## Preserved milestones

- `bcc68cf5`: GR2 wildcard matching and GR3 fallback contracts.
- `17009529`: GR1 exact provider-record boundary, environment conversions and FSAA probe outcome.
- `320daea8`: named-manager inventory, lifecycle/numeric/logging contracts and ten independent vector groups.
- `2523f3f3`: logging correction (true/false flags and date/time header) and prior handoff, preserved as ancestor.
- `2359bd56`: primary native evidence for group defaults, owner-inventory entry and V5 citations.

## Whole-owner checklist

This is the named-method contract checklist for the coordinator's `cSC4ConfigRuleManager` target
(15,119 bytes), not a fresh measured byte-coverage result. The address inventory remains in §12.

- CLSID, QI, interface thunk, factory: §1 identity and interface delegation.
- Constructor, destructor, deleting destructor, destructor thunk: §1 defaults, lists and ownership.
- ApplyPropertySet: §1 typed, ordered writes retaining duplicates.
- LogOptions, LogConfigInfo: §6 preference summaries, environment and renderer output.
- ApplyOptionGroupSetting, GetOptionGroupAutoSetting: §§4.1/6 bounds, recommendation versus application, defaults.
- RuleMatchesAll, RuleMatchesAny: §4.2 flags, signed integer thresholds, wildcard polarity and short circuit.
- GatherInformation: §§2–2.1 provider acquisition, conversions and flag assembly.
- ParseConfigFile: §3 paths, IDs, symbols and successful ownership transfer.
- LogConfigurationToFile: §6 regather, explicit path, return value and failure behavior.
- ApplyRules: §5 depth traversal, recommendation/property actions and persistent state.
- SetFallbackRules: §7 exact three-rule fallback.
- Init, SetOptionsFromConfiguration: §§3/6 initialization, evaluation and logging sequence.

Supporting parser/rule/option-group/vendor callbacks and AddOptionToSet are inventoried in §12.
The last unverified group-default constructor is now supported by primary native evidence; no
missing body remains for this bounded verification task.

## Validation and residuals

- `git diff --check cb178caa HEAD` and working-tree `git diff --check`: passed, including prior logging correction.
- `python tools/questions.py check`: passed. `docs/specs/questions/graphics-rules.md` has no open entries; no new question or question-index change was needed.
- Ten independent **prose** vector groups exist (V1–V10); none was executed against an implementation. They cover thresholds/write order, flags, wildcards, nested rules, recommendations, persistent hardware clearing, conversions, fallback, vendor latch and logging/failures.
- Cargo, clippy and ctest: n/a for this docs-only lane. Game runs and native-shadow comparison: n/a. No implementation, verification or replacement coverage is promoted.
- No engine files were read or edited. No tools, ABI tables, unreconciled rows or main-checkout files were edited. No ABI exports changed and no ABI proposals were produced.
- Evidence queries used the read-only export and original executable; REA was unnecessary. The earlier round's q.py Bash access failure was bypassed with direct PowerShell/export queries.

Implementation still needs the explicit hardware/OS provider boundary, filesystem/preferences access,
and the shared `nSCRes` parser dependencies described in §12. Extra expression operators and lexical
escapes are not inferred from shipped scripts. Those boundaries and the absence of executed fixtures
remain visible; publication of these contracts is not evidence of runtime parity.

Next: the coordinator can publish the detached commits to the reserved lane branch and merge the
docs queue entry. Implementers can use §§1–12 and V1–V10 for the core work; there is no additional
RE work requested by this bounded task. Queue path: `build-runs/merge-queue/re-graphics-rules-completion4.json`.
