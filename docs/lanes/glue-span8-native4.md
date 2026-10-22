| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span8-native4: owned span8 native adapter | `glue-span8-native4` (engine lane, detached HEAD) | in progress | Implement checked adapter and bounded native fixture bridge, gate and game checks. |

## Handoff
- Goal: deliver checked span8 CAPI and opt-in native type-2 bridge using reviewed SR9 contracts.
- Base: 1e4c8d55, reviewed SR9 rows applied in base commit; preserve SR8-2/3/4 gaps.
- Done: baseline cargo/clippy and full native ctest 68/68 passed (logs build-runs/qbuild-*, fc4verify-*).
- Done: 8a5a3d73 initial checked C adapter/native bridge milestone (by call30). Focused quick PASS 531 CAPI tests and plugin compile.
- Done: constructed native keys use exact §10.7 buffer +0x24/key +0x28/flag fields, restored after comparison; context only via public setters. Direct and native each expect 160 changed/exact cases, 156 kernel combinations; per-kernel counters separate exact/diff/refusal/fallback/resync.
- Done: new span_render8.rs rustfmt check PASS. Checking lib.rs recursively reports pre-existing drift in untouched advisors.rs/other modules; no mass formatting.
- Done: full all PASS cargo4952/clippy0/ctest68, but administrative commit changed tree during gate; stable rerun required.
- Done: s8n4-own-45d931a9-20261022a clean2032-01-15/ref0. Direct160/160 and native160/160 exact+changed; all156 kernels in each, guards/padding/faults/refusals/fallback/resync0. All locks restored and owned refs0; shipped/other0 (no credit).
- Frozen DLL: build-runs/dll/span8-45d931a9/FreeCity4.dll SHA256 B02D0A03D8CEB8CC832D2D5360A754A65B0B198964644293704AD1D2F78F6CBB.
- Next: commit this verdict milestone; stable all and unique current40+span8 replacement request (prepared build-runs/span8-c40.request); no requests outstanding yet. Wait script build-runs/wait-span8-verdict.py (no qwait).
- Shell q.py cannot start Bash (E_ACCESSDENIED); use capped native PowerShell commands.
- No game requests outstanding. No ABI-table edits, no default promotion, no shared Git ref writes.
