| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-shadow5b: AD19 advisor record shadow | Codex / glue-advisors-shadow5b | ready: cargo 6536/0, clippy clean, CTest 69/69; clean 2032 game, advisors 73/73 with AD19 fixups | Coordinator: credit coverage, append advisors_shadow=1 to regress shadows profile, regenerate questions index. |

## Report (2026-10-23, AD19 round)

**adv-shadow5b-20261023-r3-ad19-2032 73/73**. [Run record](../runs/adv-shadow5b-20261023-r3-ad19-2032-332.md). Konradshohe reached **2032-01-05**, verdict **clean**, `ref_warnings=0`; foreign exceptions, watchdog stalls, RTC and overruns all zero. All **73** dated advisor shadow samples have `mismatches=0`. This is exact under the coordinator's AD19 Read-fixup comparison.

Recorded the coordinator harness decision under AD19 and moved the original question to [answered/advisors.md](../specs/questions/answered/advisors.md). The shadow accepts a clean advice record only when the entire rewritten record equals the captured record after exactly the AD18 `F OR 0x10` change, counting each accepted record as `fixups=N`. Every other byte, field, framing and reference remains exact. Native flags, captured bytes and core Read/Write remain unchanged. There is no native Read, simulation or replacement factory. `advisors_shadow=1` remains default off. MySim records remain outside this record boundary, with their six identities retained. **C ABI exports/version did not change.**

Implementation **69c5ef9f**; fixture correction **4ea57879**. Native graph tests cover clean city-situation/plain advice, high flag bits, already-dirty records, extra bytes, rejected versions, unchanged native flags, no native Read and balanced acquired references. Advice flags bit 2 must agree with the advisor record's active byte.

Stable final source passed `qbuild all`: **6536 passed / 0 failed / 0 ignored**, Clippy **0 diagnostics**. The command submitted native verification through the coordinator build queue: **qbuild-20261023-224452-build-ctest**, build **51/51** edges (**2.9 s**), CTest **69/69** (**82.2 s**), total **87.2 s**. No duplicate explicit verify was launched. Logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`.

First gate **qbuild-20261023-224153-build-ctest**: Rust/Clippy passed, native build **52/52**, CTest **68/69**. The high-bit fixture cleared active bit 2 but serialized the advisor active byte as 1; the comparator correctly rejected that additional difference. Failed advice Read also caused a dependent advisor mismatch. Corrected fixture flags to `0x80000002` and tested rejected advice streams in isolation, then repeated the full gate once. Full initial native evidence: `build-runs/fc4verify-{build,ctest}-ad19-initial.log`.

Final game DLL SHA256: `9e8ab93dc88c1755dfcac3445513e9a7fc832566f0957c63ab8ee92e62a99083`. Turbo, `disable=advisors,mysims`, end date 2032-01-01; simulation **110.109 s**, wall **127 s**. **1468** accepted AD18 fixups across **73/73** samples, **0–28** per sample; **zero other differences**. Load: **858 records / 57833 bytes / 0 fixups / 0 mismatches**. Final month: **861 records / 57914 bytes / 24 fixups / 0 mismatches**. Exact final monthly line:

```text
advisors shadow 2032-01-01 month: records=861 bytes=57914 MySim_identities=6 fixups=24; mismatches=0
```

Historical strict runs remain recorded: turbo `adv-shadow5b-20261023-r1-2032 1/73`, normal `adv-shadow5b-20261023-r2-normal 1/4`, all differences precisely AD18. AD19 supersedes their strict-shadow credit restriction for this harness only, without introducing a separate live import. The final run has no remaining differences and needs no normal-speed diagnosis. Full verdict/log: `build-runs/game-results/adv-shadow5b-20261023-r3-ad19-2032.{verdict,log}`. All operations completed; no request outstanding.

No new questions; AD19 answered and archived. Coordinator follow-up: regenerate the questions index, credit verified coverage, append `advisors_shadow=1` to the regress shadows profile. This lane cannot open or edit `tools/` under the clean-room rule. No coverage/regress tools were edited. Prior-round implementation and validation details remain at commit 30c2cf74 and in the historical run records.
