| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-command-system-controls4: bounded native-owner OFF/ON controls, original final3/3 | Codex / `verify-command-system-controls4` (detached HEAD) | ready within supported optional-owner profile; OFF/ON clean2032/fullsave/shutdown/ref0; ON7414/7414 exact,refused0,historical171 retained; source unchanged | Coordinator land docs-only bounded completion; preserve historical disable-all failures and artifact distinction. No further phase/reload/owner expansion. |

## Final report: original FINAL3/3 (2026-10-23)

Exactly two new sequential requests completed on the copied frozen **train108 DLL/PDB**.
OFF actually finished clean/full-save/shutdown before ON was submitted. ON observed
**7414 checked,7414 exact,0 mismatches,0 refusals**; OFF earns lifecycle credit only.
The unchanged verifier separately reports **171 historical logical-count differences**.
No new comparator, owner, registration, execution, source fix or fourth phase was introduced.

### Matched supported profile and retained requests

The profile comes from the copied public `verify-cmdline-native4.md` final report,
not its binary or verdicts. Both requests literally specify empty `enable`,
`disable = budget`, `refaudit = 1`, `popup_dialogs = 0`, `autotest_end = quit`;
Konradshohe,seconds420,timeout600,end2032-01-01,turbo1. The shared script is exactly
`2031-12-01 save` followed by `2031-12-15 mark commands-controls-fullsave-complete`.
Audit confirms that only label and `commands_verify` differ between the actual
retained request files. Both actual enabled-subsystem lists are exactly **simulator**;
budget is disabled and every optional replacement remains off/native. This is a
supported optional-owner profile, with our simulator, not an all-original simulator.

| Unique label / run record | Verifier | Verdict / date | Save completion BST / game date / full-save duration | Real simulation / wall / queue wait |
|---|---|---|---|---|
| [commands-controls4-r3-off-train108-20261023-a](../runs/commands-controls4-r3-off-train108-20261023-a.md) | OFF | clean /2032-01-07 | 02:28:59.036 /2031-12-19 /11000ms,ok1 | 44.141s /56s /4s |
| [commands-controls4-r3-on-train108-20261023-b](../runs/commands-controls4-r3-on-train108-20261023-b.md) | ON | clean /2032-01-19 | 02:31:34.847 /2031-12-10 /11031ms,ok1 | 41.718s /51s /3s |

OFF queued02:28:04,started02:28:08; ON queued02:30:44,started02:30:47, both slot1.
OFF mark ran02:28:59.036 on2031-12-19; ON mark ran02:31:34.923 (scheduled2031-12-15,
no separate run-on date printed). Both scripts completed all2actions. End-date
overshoot is reported as measured, not normalized to2032-01-01.

| Shutdown evidence (BST) | OFF | ON |
|---|---|---|
| Simulator Shutdown | 02:28:59.472 | 02:31:35.545 |
| PreAppShutdown | 02:28:59.486 | 02:31:35.564 |
| PostAppShutdown | 02:28:59.512 | 02:31:35.590 |
| PostSystemServiceShutdown / final verdict | 02:28:59.585 | 02:31:35.674 |

Both RequestQuit(false,false) calls returned1. Each final verdict has
foreign_exceptions0,watchdog_stalls0,rtc0,overruns0,ref_warnings0. Each harness summary
has Rust panics0,structured exceptions0,shadow INCOMPLETE0. Both watchdog startup
lines retain stall5000ms,no-OnTick15000ms,heartbeat10000ms. No guard was overridden
or suppressed. One warning in each run is retained verbatim: unknown config key
`commands_verify` (ignored). This is the generic config-key diagnostic; the verifier's
independent `SettingInt("commands_verify","FC4_COMMANDS_VERIFY",0)` reader remains
unchanged. OFF emits no comparisons, whereas ON actually runs and logs them.

### Actual native authority and denominators

At02:30:53.722, ON acquired the app-owned public command service03054614 through
service **EB903A32 / IID CB90253A**, vtable **SimCity 4.exe+0x6D17A0**. The unchanged
slot5 factory created independently owned ORIGINAL sets1A8A2E54,1A8A2E94,1A8A2ED4,
all vtables **SimCity 4.exe+0x6D16C8**. Eleven subsequent parser-returned set
acquisitions reused address1A8A2E54 after each release; these are eleven acquisitions,
not eleven simultaneously distinct addresses. Refaudit at shutdown records sets
acquired14/released14/net0,last-reference releases14,created temporaries14; service
acquired1/released1/net0,remaining native count6 after our release. Thus the measured
authority is original native command ownership, independently of simulator/budget.
Unavailable/unsupported/our-owner refusal paths did not occur and earn no equality.

| Actual assertion family | Attempted / compared | Exact | Mismatch / refusal |
|---|---:|---:|---:|
| Owned-set assertions | 3708 /3708 | 3708 | 0 /0 aggregate |
| Parser and prerequisite assertions | 3706 /3706 | 3706 | 0 /0 aggregate |
| Total Eq invocations (logged checked) | 7414 /7414 | 7414 | 0 /0 |

The unchanged implementation logs phase totals rather than independent invocation
counters for every native getter/mutator. Its nine-assertion `Check` shape permits
the following explicit derivation from the observed totals; these are **comparison
field denominators**, not extra native method-call telemetry or independent fixtures:

| Compared output / producing method | Sets | Parser | Combined exact denominator |
|---|---:|---:|---:|
| count / GetParameterCount | 412 | 407 | 819 |
| first / GetFirstParameterID | 412 | 407 | 819 |
| next / GetNextParameterID | 412 | 407 | 819 |
| presence / GetParameter or writable operator[] | 412 | 407 | 819 |
| type / GetType or missing sentinel | 412 | 407 | 819 |
| native elements / GetCount or absent zero | 412 | 407 | 819 |
| value / typed scalar getter or zero/default | 412 | 407 | 819 |
| string length / GetValString or nonstring zero | 412 | 407 | 819 |
| string bytes / GetValString or empty bytes | 412 | 407 | 819 |
| ConvertStringToCommand return, ID, pointer presence | - | 14 each | 42 |
| GetCommandID documented prerequisite | - | 1 | 1 |

412*9=3708;407*9+14*3+1=3706. All819 snapshot records complete with refusal0.
Factory calls3 produce3 owned sets; parser calls14 produce11 acquired native sets;
the three null-output cases still compare return/ID/pointer, not fabricated snapshots.
Mutations (SetParameter,AppendParameter,SetParameterCount,RemoveParameter,
SetStatusParameterValue,writable fallback,CopyFrom) are assessed by the existing
snapshot assertions; independent per-mutator attempt/exact counters are not logged,
and no such counters are invented here. No live global dispatch/registration occurs.
elements_diffs0 and other_diffs0 coexist with historical_logical_count_diffs171:
the production NativeParameter projection is compared, while logical DTO diagnostics
are retained. GC1-GC7 stay excluded; GC8 remains the settled adapter distinction.

### Artifact and source provenance: two separate DLL identities

The new tested DLL is **45de4459b606931d384a166726a0b1df8870465af8f84fff89cb58cf781fcd36**,
18282496 bytes, adjacent PDB **3bf57b3ab7e88eda24b5c43415cb0071280f8bf86356108f3ebe5a9cdd8464ad**,
171528192 bytes, both in this worktree's `build-runs/evidence-run4`. Copied gate
source **bfb7fc14d7b70eaeba82ea9f1ee2088d9e29f761**, tree141e270dcab5e4cce1a219981ac0919dcbb5b548,
engine **fcf40905fd2ea714a9f3dfa8bd8fd9da453b9af2** matches this worktree's engine tree.
`manifest.json` records the DLL source/before/after/copy equality; `pdb-manifest.json`
at02:21:48 records the later PDB source/before/after/destination equality and matching
DLL/source identity. Its later proof supplements the earlier manifest/review's
explicit no-PDB-preservation claim. All evidence read was copied inside this worktree.

Both pair hashes/sizes were checked before OFF02:27:49,after OFF/before ON02:29:37,
again before ON02:30:33,and after ON02:32:45; source/gate provenance and absence of
outstanding local requests passed. The single blocking waiter per label accepted
the existing verdict; neither verdict was replaced, discarded or retried. The
transport returned asynchronous sessions, so final evidence was read after completion
without starting a second waiter. No request remains outstanding.

The unavailable historical DLL **17115A0C565DD4E63D8191255C47B50CA9D08561580C0F3A445BF4CD5B169396**
is not this train108 binary. Old R2 native7414/7414,clock24/current40/reload clean
results remain historical separate-artifact evidence. Old disable-all verifierON
and OFF both watchdog-stalled during2026-03-01 full save: those failures are retained,
not explained or superseded by these supported-profile successes, and establish
neither a permanent native deadlock nor a successful all-native lifecycle.
There is no new CURRENT40/reload or normal-speed credit, and no current comparison
difference requiring another run. Exactly these two new attempts consumed this
reserved completion; no further continuation is proposed.

### Checks, scope and delivery

Production command core/CAPI/verifier header diff against82376d6b exits0. Entire
engine diff against initial640ba764 exits0. This delivery changes documentation only:
no engine,ABI exports/version,maps,defaults,spec answers,shared tools,cache,policy
or gate changes. No rebuild,quick/full gate,fmt or new tests were run, as explicitly
required for frozen control-only work. Copied train108 source provenance records
Rust5329passed/0failed/23summaries,clippy clean,native190/190,CTest68/68; those are
the prior gate, not a new lane gate or game verdict. Native warnings in that copied
review remain disclosed. The optional q.py wrapper could not launch Bash (access
denied); the read-only local audit was run directly in PowerShell and passed.

Milestones: initial3bb72280,provenance/OFF outstandinge96087aa,OFF record4b0ccf47,
ON record3e16f993. Local `commands-controls-audit.py` validates actual request
matching,profiles,guards,lifecycle,denominators,native module identities,reference
balance,pair hashes/provenance,idle queue; helpers/raw evidence remain ignored under
build-runs. `git diff --check` passes. No numbered questions were needed or filed.
No other-worktree reads,RE/private-original inspection,game-install mutations,
subagents,process kills,external messages,ref updates,rebase,merge or push occurred.

Coordinator next: land this bounded docs-only completion from its actual detached
HEAD tip; retain the two label-keyed run records and raw evidence. The harness
assigned no legacy numeric run IDs, so `run` uses the unique label rather than an
invented number; coordinator owns any sequential-index normalization needed by its
table generator. Root/main/train remains the sole landing owner. No additional
command-owner coverage or all-original lifecycle acceptance is claimed.

Coordinator subsequently assigned numeric OFF/ON record IDs287/288 after an exact empty collision scan across123
run directories. Only the two run fields changed; all unique harness labels, artifact/source hashes, scripts,
raw outcomes, full-save/lifecycle evidence and historical failures remain unchanged. This is documentation
normalization, not another game run or a new source gate. Original parent2 plus final1 remains exhausted3/3.
