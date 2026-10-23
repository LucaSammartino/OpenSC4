| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propdev-pv15 | Codex, `re-propdev-pv15` (detached HEAD) | in progress | Resolve PV15 lighting-manager acquisition ABI and lifetime; doccheck |

## Handoff

- Goal: close PV15 for glue-weather-propdev-shadow5 with behaviour specs only.
- Done: PV15 answered in spec §§3.3, 8.5 and six prose vectors (§12.23–28); archived original question recovered from blocked branch.
- Evidence: renderer slot 9 0x007C1910 returns borrowed manager in EAX, ECX only, no hidden buffer/pop/AddRef. Lighting IID 0x681BD52A; IsNight AL.
- Identity: SL::OnView3DInit 0x006025C6–0x006025E7 stores the getter result used by developer PostCityInit.
- Lifetime: renderer Shutdown 0x007C71EF–0x007C7209 shuts down, releases, clears manager; retained reference does not keep it operational.
- Proposed interfaces.tsv rows: `build-runs/abi-rows-re-propdev-pv15.tsv` (no docs/abi edits).
- Next: regenerate/check question index, commit milestone, run doccheck and inspect diff; finalize report and queue.
- Sources: main checkout `C:/Users/tlori/FreeCity4`; edits only in this worktree.
- No engine reads/edits, game runs, or C ABI changes.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-propdev-pv15-r1-manifest.json`.
