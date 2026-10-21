| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-ui4: advisors AD4 (window route direct-call ABI) and AD5 (9 UI Lua entries) | `re-advisor-ui4` / (RE lane) | in progress | see Handoff |

## Handoff
- Goal: answer AD4/AD5 in docs/specs/questions/advisors.md from bytes; write rules into advisors.md §12.1.1 and §8.2/8.3; archive answers.
- Done: lane file.
- AD4 facts (asm): ctor 0x41DB50 thiscall, no stack args, RET (pop 0), EAX=this, refcount +8 = 0; slot 24 AddRefs -> 1; +0x5C = block base (vtable 0xA83548; +4 = msg-target sub-object vtable 0xA83538). Init/Show/Hide/teardown: thiscall ECX=block base, no args, RET; Init returns void (EAX junk), slot 24 tail-jumps to it. Teardown 0x41D830 AddRef self..Release self tail (EAX=Release result). Release 0x41D460: count<=1 -> deleting dtor slot 5(1), EAX 0. Win proc reaches system only via city 0x00B43CEC +0x144 -> slot 7; holds advisor ref +0x0C; subscribes via msg server 0x00B43CCC.
- AD5 facts: UI collector 0x40CAB0 vector 0xB207CC..D4, 9 entries all isGlobal=0: tool_plop_network 0x40C720, _zone 0x40C740, _building 0x40C760, _flora 0x40C780 (helper 0x40C680), tool_button 0x40C8B0, window_budget 0x40C870, _map 0x40C850, _graph 0x40C890 (helper 0x40C7A0), window_query 0x40C9B0. camera_jump/zoom are in the SIM collector 0x40B8D0. Reads of 0xB207C0: 0x40C7A0, 0x40C8B0, 0x40C9B0. REA down (no app open): asm only.
- Next: decode helper bodies 0x40C680/0x40C7A0/0x40C8B0/0x40C9B0, then write spec §8.3 + §12.1.1 and archive AD4/AD5.
