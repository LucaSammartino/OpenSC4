| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-graphics-rules4: graphics rule manager and parser | Codex / `core-graphics-rules4` (engine lane, detached HEAD) | WIP; source 97af8400; cargo 4308/4308, clippy 0 diagnostics, ctest 67/67 PASS; 31 graphics tests; GR4-GR6 open; ABI unchanged; no game/shadow/owner credit | 1. Resolve GR4-GR6. 2. Reconcile shared parser, failures and logging. 3. Repeat owner audit before credit. |

## Result and scope

Implemented the documented pure-core graphics-rule behavior in `engine/core/src/graphics_rules/` and added one module declaration at the end of `engine/core/src/lib.rs`. The implementation owns its rule/script parsing, symbols, includes, conditions, wildcard matching, group/option lists, vendor latch, property dispatch, traversal, recommendations, environment conversion, fallback construction and log formatting. No original rule-parser or manager result is a foreign input.

**State is WIP. Completed classes: none claimed.** In particular, no complete-owner credit is proposed for `cSC4ConfigRuleManager` (the task's 15119B target), `cSC4RuleParser`, `cRuleCommand`, `cOptionGroupCommand`, or `cVendorCommand`. No byte/function coverage increment, native acquisition parity, shadow, replacement or runtime promotion is proposed. The missing shared-parser grammar is required by §12 and cannot be inferred from its callback inventory. GR4-GR6 precisely identify the remaining contracts.

Only public behavior specs and interface facts were used. No native analysis, prohibited source, executable, game file or analysis-side script was accessed. No plugin/CAPI/ABI/map/tool changes; no ABI version change. No game requests were submitted: this module is unbound pure core and has no shadow key. There are no outstanding requests and no measured runtime/ref-warning result.

## Implementation and service boundaries

- `model.rs`: separate bool/int/float assignment lists, duplicate-preserving dispatch, zero-default groups, rule constructor, mask overwrite semantics, all/any conditions, signed-low32 threshold conversion and whole-byte wildcard matcher. Both wildcard operands are lowercase copies; stored data is unchanged. The default ASCII-only service explicitly rejects non-ASCII rather than inventing native non-ASCII lowercase behavior. A shared lowercase service can be supplied; wildcard decisions stay in Rust.
- `environment.rs`: typed raw measurements, exact seven-flag assembly, unsigned-to-binary32 conversion, physical/free memory four-MiB rounding, GS memory truncation, description-field mapping, sound fallback and Release build string. Constructor bogo values are +0.0; gathering never overwrites either bogo slot. The FSAA probe is called on every gather, including when software is forced. The 64-sample helper ignores alpha and requires two colored samples; actual rendering/readback remains a provider operation.
- `parser.rs`: internally owned tokenization and documented literal commands, symbols/includes, flat preorder rule records with enclosing-rule stack, dense groups/options, typed property lookup, exact enum spelling, vendor/device latch and fixed-offset ID extraction. Both root paths are acquired before command parsing. Completed lists publish only on success. Unsupported expressions fail with `Bad real number`; no invented arithmetic or Lua language is claimed.
- `mod.rs`: retained renderer through `Arc<Mutex<P>>`, same-object Init retention, replacement/drop ownership, Init versus evaluation separation, persistent recommendations/flags, unsigned option bounds, exact fallback list, explicit-path regather logging and command-line-controlled configuration evaluation. There is no RefCell. Native COM thunks/refcount interfaces are unbound; Rust ownership is exercised independently.
- `logging.rs`: owned environment/property/preference formatting, published heading/field order, application-byte preference order and paths. Filesystem, clock stamp, application preference/command-line data, OS/driver measurement and lowercase-byte services are typed inputs; no service decides rules or recommendations. Renderer access goes through the typed properties interface and the existing core render-properties adapter.

## Whole published owner inventory audit

This maps every entry in graphics-rules.md §12 to its pure-core equivalent or explicit limitation. It is a contract audit, not a native-body/byte coverage measurement. Shared allocation/relocation ownership is represented by Rust-owned lists. ABI names affected by shared-body labeling were not used to override the behavioral inventory; no ABI files were changed.

| Published entry/address | Implementation or limitation |
|---|---|
| CLSID `007CE130`; QI `007CE140`; QI thunk `007D0DA0`; factory `007D1030` | Class/interface constants are exposed; native factory, base-QI delegation and this-adjusting thunks remain unbound, with no native wrapper credit. |
| constructor `007D0CC0`; destructor `007D0DC0`; deleting destructor `007D1050`; destructor thunk `007D0DB0` | `GraphicsRuleManager::new`, default records, owned lists/logger and Arc drop/replacement; no native COM destructor claim. |
| ApplyPropertySet `007CE1F0` | `PropertySet::apply`: bools then ints then floats, insertion order and duplicates, continue after setter failure. |
| LogOptions `007CE260` | `logging::options`: property dump then application-byte summary in published order; exact layout boundaries GR6. |
| LogConfigInfo `007CEF40` | `logging::configuration`: date/time, flags/strings/numbers in table order; exact log encoding/separators/nonfinite spelling not credited, GR6. |
| ApplyOptionGroupSetting `007CE870`; GetOptionGroupAutoSetting `007CE8D0` | Corresponding manager methods; out-of-range no-op/zero; no flag or recommendation update on explicit apply. |
| RuleMatchesAll `007CE900`; RuleMatchesAny `007CEA80` | `Rule::matches`: flags/lower/upper/positive/negative order, overwritten flags, duplicate numeric/string entries, empty all/any distinction; NaN follows §4.2 pending GR5. |
| GatherInformation `007CF780` | `Environment::gather` plus explicit information/probe provider; no claim of real hardware acquisition parity. |
| ParseConfigFile `007CFE10` | `parse_config_file`, `card_ids`, `RuleParser`: default identity before result, >26-byte test, offsets 8/22, prefs symbol, success-only list transfer; grammar/file semantics GR4, malformed inputs GR6. |
| LogConfigurationToFile `007D0420` | `log_configuration_to_file`: regather even on failed open, explicit path, bool open result, environment only, no parse/evaluation/options; write/close failure contract GR6. |
| ApplyRules `007D10B0` | `apply_rules`: depth-stack traversal, child skip, sibling chain stop, partial continuation, properties then source-order recommendation actions, one-way software flag clear, persistent state between calls. |
| SetFallbackRules `007D13C0` | `set_fallback_rules`: software conditional, unconditional hardware alternative, depth-1 inclusive two-stage child; replaces rules only, no implicit options or evaluation; lookup/allocation failure GR6. |
| Init `007D1890`; SetOptionsFromConfiguration `007D18E0` | `init` has no success return and does not evaluate; stores errors separately and installs fallback on parse failure. `set_options_from_configuration` evaluates stored state even with disabled/failed logging and does not regather. |
| default group `007D0410`, fill caller `007E5549`; resize `007D0FD0` | `OptionGroup::default` and `model::grow`: recommendation 0/empty options, dense fill; `UINT32_MAX` growth rejection tracked in GR6. |
| rule constructor `007D0500` | `Rule::default`: unknown name, depth 0, nonpartial/all mode, empty conditions/actions. |
| parser constructor `007E4DE0`, continuation `007E4DE9`; destructor `007E5030` | `RuleParser::new`, owned parse state and Rust drop; service is borrowed under the manager's retained renderer lifetime. Native parser reference-count wrapper remains unbound. |
| parser RegisterCommands `007E3DF0`; ParseBegin `007E5580`, thunk `007E5620` | Internal command dispatch and fresh parse state; rule/group/vendor roots and renderer borrow. Complete shared-parser equivalence is blocked by GR4. |
| rule constructor `00677290`, RegisterCommands `007E3C60`, Parse `007E4550`, EndBlock `007E3DD0` | `begin_rule`, command dispatch and rule block stack; flat append at current depth, restored enclosing rule after end; previous emitted-name behavior. |
| DoHas `007E3ED0`, DoHasNo `007E3F90`, DoAtLeast `007E4300`, DoAtMost `007E43B0`, DoStringMatch `007E5160`, DoStringNotMatch `007E5400` | Internal condition commands target the enclosing rule, with exact enums and typed condition lists; complete expression parsing remains GR4. |
| rule DoOption `007E4180`, DoProperty `007E4250` | Internal action commands append recommendations and typed property records to the current enclosing rule. |
| option-group RegisterCommands `007E3B70`, Parse `007E54C0`, EndBlock `007E3B50`, DoOption `007E4910`, DoProperty `007E4490` | Group/option frames, dense growth and reopening append. Group end clears selection; a property without option is rejected locally, without claiming native recovery for this structural violation. |
| vendor RegisterCommands `007E3C00`, Parse `007E4800`, DoCard `007E4730` | `begin_vendor` and card command: reset active vendor before testing, ID filtering, matching-card first-found latch. |
| AddOptionToSet `007E4050` | `add_property`: bool then int then float lookup, typed parsing and exact unknown-property error. Full native value grammar is GR4. |

## Independent verification

31 graphics-rule unit tests cover all ten published vector groups and additional independent boundaries. The vectors are proof-derived expected results, not native captures. V1's NaN sentence conflicts with §4.2 and is tracked as GR5 rather than silently reconciled.

| Published group | Checked results |
|---|---|
| V1 | 2.9/-2.9 truncation, signed-low32 wrap, no binary32 threshold rounding, typed duplicate setter trace; signed64 invalid conversion boundaries. |
| V2 | Repeated flag last-win, any-mode agreement, empty-any false, empty-all true. |
| V3 | All published wildcard rows and inverse polarity, storage unchanged, literal brackets/backslashes, whole-byte matching, lowercase service separation. |
| V4 | Parent writes 1,4 before child write 8/9; restored parent conditions; failed parent skips descendants; multiple depth-stack levels and partial siblings. |
| V5 | Dense default groups/options, recommendation-only evaluation, explicit apply, reopening append, unsigned out-of-range no-op/zero. |
| V6 | Force/undo/sw traces, no hardware restoration on false assignment, repeated evaluation persistence, gather restoration. |
| V7 | Flags 0x5F/0x5E and backing-store 0x7E, 260/128 MiB, GS 3/0 MiB, unsigned high-bit conversion, description/sound fields, untouched bogo values, alpha-only/0/1/2-sample probe outcomes. |
| V8 | Software stages 0/8, hardware stages 1/2, exact fallback property trace and false-then-true second-stage writes; no implicit group creation or evaluation. |
| V9 | 27-byte versus 26-byte cardVersion extraction, first-card latch, new-vendor reset, fallback identity on failure, unmatched card behavior. |
| V10 | Explicit logging regathers without evaluation, failed open retains gathered state, writeLog off still evaluates without gather, parse failures never publish partial lists, Init fallback. |

Additional checks exercise exact enum spelling, quoted comments/CRLF and whole-token symbols, include/root parse ordering, local malformed-input rejection, bool/int/float lookup precedence, setter-failure continuation, same-object/replacement/drop retention, persistent recommendations and successful list replacement. The greedy wildcard algorithm is compared against an independent dynamic-programming recurrence for **28,985** pairs (85 values × 341 patterns), including literal `*`/`?` in input values.

Measured checks, in order:

1. Coordinator's stable native train baseline was accepted as instructed; no full pre-edit baseline was repeated.
2. First focused `qbuild quick`: core **2436 passed / 0 failed**, dependents compile-only passed, native **206/206** edges, overall **154.4 s**; implementation commit `16091081`, gate record commit `94c8528c`.
3. Reviewed/formatted source `97af8400`: focused `qbuild quick` core **2441 passed / 0 failed**, dependents compile-only passed, native **4/4** edges, overall **133.6 s**.
4. `qbuild fmt` **PASS** for all six new Rust module files. The combined seven-file check exits 1 with **126 diff lines** in shared `lib.rs` (existing ordering/wrapping plus the required additive module placement); it was not mass-reformatted. The initial relative-path fmt invocation failed to locate files; the absolute-path rerun produced these results. `q.py` could not launch Bash (`E_ACCESSDENIED`), so targeted formatting of new files used native PowerShell.
5. ONE final stable `qbuild all` at committed source `97af8400`: workspace cargo **4308 passed / 0 failed / 0 ignored**; clippy **exit 0 / 0 diagnostics** with warnings denied; native build **2/2** edges (**0.5 s**); CTest **67/67**, **0 failed**, **83.1 s**. Final qbuild verdict **PASS**. No source edits followed that gate; only this report/queue metadata.

Full logs remain in this worktree's `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-quick-test.log`, `fc4verify-quick-deps.log`, `fc4verify-build.log` and `fc4verify-ctest.log`. Generated `engine/tests/abi_entries.inc` was reported unchanged and was never edited. Git's attempted packed-refs lock emitted permission errors during commits, but detached HEAD commits succeeded; no refs/config/hooks or maintenance were changed.

## Questions and next work

All questions are appended in `docs/specs/questions/graphics-rules.md`; no other spec file was edited. The generated question index was left to the authorized coordinator/spec workflow (engine lanes cannot run `tools/` scripts).

- **GR4:** exact shared lexer/expression/command/value grammar, symbols/includes and root-registration 1/0 file semantics; whether any Lua command belongs here. Current code intentionally implements only the documented literal subset and labels local rejection policies. This blocks a complete parser/manager claim.
- **GR5:** resolve the direct §4.2 versus V1 NaN contradiction. Current implementation and independent test follow the literal §4.2 all-pass/any-no-match rule.
- **GR6:** failure/lifecycle/malformed description/hex/allocation boundaries, cardIdentity regather behavior, byte-exact logging layout/nonfinite formatting and post-open write/close failures. Current literal policies are commented in code and recorded in the question.

Next: obtain those public contracts, reconcile the local policies and add independent expected vectors for the newly defined grammar/failure behavior, then repeat this whole inventory audit and the required gate before proposing any completed-class credit. Native binding/shadow/replacement would require a separately authorized glue task; none is implied by this WIP core report.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-graphics-rules4-r1-manifest.json`. This reserved worktree remains detached; the supervisor publishes commits to `core-graphics-rules4`. No branch switch, merge, push, restart or subagent was used.

## Coordinator phase review

Bounded preparation and diagnostic source publication is approved for train83; the worker result remains incomplete. GR4-GR6 remain unanswered owner contracts. No complete-owner, exact shadow, default or replacement credit is granted. The original worktree is parked awaiting answers, and its existing round budget is retained. The train must pass the full native gate and current default union before landing.
