# Runs: glue21fix2 (the PL17 CreateLotObjects fix)

All runs: city Konradshohe, `turbo = 1`, `end_date = 2032-01-01`; the game queue. "checked" = the
`-DFC4_CHECKED=ON` build (RTC failures logged with call sites).

| Run | INI / DLL | Verdict | Result |
|---|---|---|---|
| glue21fix2-gate-1 | `enable = propmgr, lotcfg`, three shadows; debug | exited -1 | The PL17 AV at `SimCity 4.exe+0x60D4B5` is GONE (slot 27 fills the caller's `SC4Vector`); a new in-dll int3 (CRT `failwithmessage`) right after the civic `AddAgent` burst. |
| glue21fix2-rtc-1 | same; checked | corrupt | RTC ERROR #0 named: "ESP not preserved" at `cFC4PropManager.cpp:416/417` — `SetPosition` called through the prop-occupant primary vtable whose slot 9 is `GetOrientation` (pop 0). Chain: game `CreateProp` (`exe+0x20CFE8`) → `fc4_prop_manager_add_prop_new` → `add_street_prop` → `set_position`. Then an AV writing `0x00AA48F8` (the desync's downstream damage). |
| glue21fix2-rtc-2 | checked after the QI normalization | exited -1 | 100 `over-release` warnings at `CreatePropInstance` (the factory reference released before it was audited) — fixed by `FC4_ACQUIRED(prop, "GetClassObject")` first. Then the lotcfg vector<bool> AV at SimNewDay. |
| glue21fix2-rtc-3 | checked, audit fixed | exited -1 | ref_warnings = 0, rtc = 0; the remaining fault is the lotcfg configuration-lifetime AV (independent of propmgr). |
| glue21fix2-iso-lotcfg-1/2 | `enable = lotcfg` only; debug (2 = with in-dll AV stack capture) | exited -1 | Same AV at SimNewDay 29 with propmgr OFF — the lotcfg lifetime issue is independent. Stack: core `simulate_agents` → deliver → game growth dev (`exe+0x304C1E`…`0x30975F`) → `cFC4LotConfiguration::IsCompatibleWithPurposeType:191` → `std::vector<bool>` reading 0 (a deleted wrapper; `edi` = the dead configuration). |
| glue21fix2-iso-lotcfg-3/4/5, isoref-1..4 | `enable = lotcfg`, per-op ref tracing | exited -1 | The ref ledger measured: creation +1 (catalogue, via core `add_configuration`), +1 raw per slot-20 map rebuild (`InsertConfig`), +1 per set-query insert (§B5.2 — was missing, added; duplicates kept per quirk 12). The game's growth pass then releases 4 references per configuration (release site `exe` ILT `0x00709798`) and the wrapper deletes under the catalogue. Slot 12 (release every element once + empty the set) implemented per §B7. Filed as PL18. |

## Final gate at `1b840d70`

qbuild all: cargo test 2860 passed / 0 failed, clippy clean (0 diagnostics), build 3/3 edges,
ctest 100% (58/58). Not yet game-clean: the PL18 configuration-lifetime fault stands between the
run and a `clean` verdict; the merge-queue entry is `wip` until PL18 is answered and the run ladder
reaches `clean` with the three shadows exact.
