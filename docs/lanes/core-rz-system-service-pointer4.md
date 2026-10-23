| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rz-system-service-pointer4 (system-service pointer utility, system-service-pointer.md §§2–4) | coordinator (Claude) / `core-rz-system-service-pointer4` (engine lane) | ready: `fc4_core::rz_sys_serv_ptr`, 14 tests transcribing §5 traces; gate cargo 5740/0, clippy clean, ctest 68/68 | - |

## Report

- `engine/core/src/rz_sys_serv_ptr/`: the stored wrapper (construct by framework lookup, status ignored), the
  §3.1 release that keeps the field, the §3.2 clear-then-release that returns the field for a foreign output,
  the §3.3 add-then-release-then-store assignment, the §4.1 temporary lookup helpers, and the §2.2 identity
  table (19 C40 + 3 C37) plus the two temporary identities. `Framework` and `References` are the foreign calls;
  `References` is told the field's contents during each call, which is what the spec orders.
- Tests: S01–S06 (with Registry, WWW and WinIME words), A01–A05 with the synthetic counter fixture, L01–L09,
  D01–D04, R01–R03.
- Not done: D05–D07 and R04/R05 belong to their callers' owners (cGZWin, WinIME, HitChannel), which are not
  implemented here. Not wired into the plugin. SSP2 (source attribution) and SSP3 (exceptions/reentry) stay open.
