| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-nativebinding4: LH1/LH2 native binding | Codex / `re-lua-nativebinding4` (RE docs, detached HEAD) | ready: d9df37c5, a7193bbb; LH1/LH2 answered; docs validation passed; cargo/clippy/ctest and game n/a; ABI unchanged | 1. Coordinator reviews owner insertion timing before any activation. No further RE phase requested. |

## Report

Answered only LH1/LH2 in lua-host §§1.2, 1.4.1 and 6.3.2. Original questions are preserved in
`docs/specs/questions/answered/lua-host.md`, marked DIFFERS, and the question index is regenerated.
Commits: d9df37c5 establishes identities/typed slots; a7193bbb completes lifecycle and ownership.

### Native findings

- Actual ScriptServer CLSID `CBD101A9`, registered by cSC4COMDirector at `0046021A–00460227`
  with factory `008B10D0`; service ID `4BD10133`, IID `4BD10129`, signed priority `FFF0BDC1 = -999999`.
- AddSystemServices requests the factory first (`00450813–0045082F`), directly constructs on false,
  QIs SystemService, adds it to the registry and releases temporaries. It does not call Init.
- Framework Init walks signed priorities descending. ScriptServer Init precedes CommandServer
  priority -1000000 Init, although CommandServer was constructed/registered first. Registration
  makes a service acquirable before its Init. Tick subscription is separate from registry insertion.
- CommandServer temporary helper `00562DA0` is mislabeled as a Lua operation in the export.
  Its actual public acquisition is framework GetSystemService(`EB903A32`, `CB90253A`, out).
- Lua-generator view `2BD144DF` has borrowed AsIGZCommandGenerator at +0C and bool Init with
  CommandServer pointer, Lua5 pointer and const string reference at +10. Generic view `0B90251F`
  has bool Shutdown at +10. QI acquires references; sibling conversion does not.
- Generator retains CommandServer/main-thread references and copies `sc4app`; caller destroys
  temporary service reference/string after Init. Owner shutdown ignores foreign Shutdown status,
  releases generator then closes/releases Lua. Missing-generator failure latches; retry needs Shutdown.
- Command handlers acquire per operation; script windows retain service references and tutorial
  retains Lua. Registry edits do not retarget these caches. No patching/absolute-call workaround specified.

### Binding limits and coordinator action

Factory dispatch is a real pre-creation seam, but this trace does not establish a public plugin callback
guaranteed early enough, or a duplicate-class override policy. A duplicate service-ID Add fails.
No safe late live-substitution recipe is supported. The native Lua-generator slot contract is now exact;
its Lua-specific interface declaration is absent from the inspected SDK. Public generic generator,
CommandServer and Lua declarations are available. Keep owner installation timing distinct from
foreign-endpoint bindability; these findings do not establish activation or replacement success.

### Validation and evidence

Nine independent discriminators cover IDs, creation/Init timing, pre-Init acquisition, retained caches,
latch/retry, null CommandServer, borrowed/QI references, temporary lifetime and shutdown order/status.
They are specification inputs, not executed tests. `python tools/questions.py index` and `git diff --check`
passed. Cargo/clippy/ctest/game/shadow n/a for this docs-only lane. No C ABI exports or docs/abi tables
changed; no ABI proposal rows needed. No engine/tests/code reads or changes, runtime/default/coverage claims.

Queries used main-checkout re_query first, then bounded decomp excerpts and primary byte verification;
ignored `build-runs/native_tables.py` reads PE vtable words. Export xrefs alone missed the instruction
reference to the factory; its native PUSH/call sequence proves registration. App decomp failures were
handled with instruction export; REA was unnecessary. Public SDK read only from main external/gzcom-dll.
The q.py Bash wrapper failed E_ACCESSDENIED, so local PowerShell was used. Git commits succeeded despite
a nonfatal denied shared packed-refs lock; detached HEAD and shared refs/config were left unchanged.

Guard after substantive answer: context 137028/150000, within budget. One guarded native-binding phase;
no subagents, network, restart, merge, push or extra phase. Ignored evidence and commit messages stay in
this worktree's build-runs. The supervisor publishes detached commits to the reserved lane branch.
