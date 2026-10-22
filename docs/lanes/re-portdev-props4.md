| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-portdev-props4: PD12 native seaport prop switching | `re-portdev-props4` (RE; reserved detached worktree) | in progress | Trace native enumeration, ownership, property lookup and SetState; publish PD12 contract. |

## Handoff
- Goal: answer only PD12 in port-developers section 4.4 with native evidence and independent vectors.
- Done: scope committed e886822d; PD12 contract and twelve independent vectors written in section 4.4.1; question moved to answered.
- Next: regenerate question index, commit proof/contract, check diff, publish ready report and queue.
- Scope: docs only; never read engine; no ABI table edits, subagents, branch changes or game runs.
- Existing evidence: glue-portdev4 reports 144/144 shadow exact; replacement clean/ref0 to 2032; prop host stub returns 0.
- q.py cannot start Bash (E_ACCESSDENIED); use native PowerShell and bounded output.
- REA returned target_unavailable (no open app); fallback used local export assembly with bytes plus direct PE vtable words.
- Proof: 00701a60 calls 00701190; filter 007010e0 tests type, inclusive x/z bounds and holder HasProperty, not property value.
- Boundary: occupant-manager slot40 lists owned occupant references; slot48 releases/clears; cISC4Prop IID e9793a65 slot11 is void __thiscall(state32), pop4.
- Count is adapter instrumentation only; native no count; preserve provider order/multiplicity; null QI skips, missing providers are unchecked preconditions.
- ABI proposal artifact: build-runs/abi-rows-re-portdev-props4.tsv (vtables.tsv format; canonical prop slot11 return correction only; not applied).
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-portdev-props4-r1-manifest.json.
