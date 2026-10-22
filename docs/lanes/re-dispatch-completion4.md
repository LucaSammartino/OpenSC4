| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dispatch-completion4: DP10–DP13 native contracts | `re-dispatch-completion4` / (RE lane, detached worktree) | in progress | 1. Trace native view, dispatch vehicle, pylon and air source contracts. 2. Publish supported answers and independent vectors. |

## Handoff
- Goal: resolve only DP10–DP13 from public measured gaps and original export/decomp evidence; no engine reads or edits.
- Base HEAD: 8ec42a46; worktree clean at start, detached as requested.
- Reviewed opt-in phase is on base; coordinator grants no owner/replacement credit pending these contracts.
- q.py Bash wrapper failed with Bash/Service/CreateInstance/E_ACCESSDENIED; use bounded native PowerShell local reads and configured Python RE tools.
- Done: DP13 contract in dispatch §4.5.1 / protection §8.4; archived answer and independent vectors. Both air tables point to 006E0010, CopyOccupantList 006DFC70 AddRefs; node clear 00413E60 does not Release.
- Next: DP10 view has direct ctor 0046F120, primary table A88248; QI 0046B500 exposes message target +0xC and falls back to cRZUnknown at +4. DP11 direct pylon ctor 005F1730; DP12 BeginDispatch source slot11 is not fully decoded yet.
- No game requests, ABI proposals or tests yet.
