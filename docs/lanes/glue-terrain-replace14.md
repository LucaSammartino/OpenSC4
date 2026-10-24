| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain-replace14 (terrain replacement interface audit) | `Codex` / `glue-terrain-replace14` (engine lane) | in progress: commits 2e7504714, 69cd3b6be; cargo 7484 passed / 0 failed; clippy interrupted; native/game pending; TR44 filed; ABI unchanged | (1) Run final stable-source all and queued verify; (2) measure replacement and shadow to 2032; (3) resolve remaining terrain contracts. |

## Handoff
Goal: make terrain replacement pass the house replace run through 2032 and retain exact shadows.
Done: 2e7504714 corrects slot 66 per terrain.md §15.5.6 to return the view sub-interface at base+0x0C, preserving null.
Done: 69cd3b6be requests factory IID 0x6771477D and stores base per §15.5.1; acquisition/release audited; replacement seam invoked by the terrain native test.
Measured: qbuild all began before the companion edit; workspace tests and doc-tests completed, summed 7484 passed / 0 failed.
Gate: clippy still compiling after a long contended build; interrupted for requested context wind-down. Original session 67957 collected terminal exit 1. No native verify or game request outstanding.
Evidence: build-runs/qbuild-initial-test.log and qbuild-initial-clippy.log preserve this incomplete check, separate from future final-source checks.
Next exact step: run python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/glue-terrain-replace14 all on stable source; use queued qbuild verify for native build/CTest, never direct fc4verify.
Then: shadow on originals (terrain_shadow=1, turbo, end_date=2032-01-01), replacement trial and house profile; choose unique labels, one outstanding request.
Waiting helper: build-runs/wait-terrain-verdict.ps1 uses one filesystem-event wait and accepts an already completed verdict; no qwait per transport instruction.
Question: TR44 in docs/specs/questions/terrain.md requests the reported crash receiver/slot and slot 54 render-object identity/lifecycle.
Known remaining contracts: native view Init is not called; surface-water direct array layout and other §15.5 requirements remain unaudited. Do not claim replacement success from this pointer correction alone.
Clean room: no RE sources, forbidden tools/, game executables or main checkout edits accessed. House profile edits and coverage credit remain coordinator work.
No game run records or exactness counts yet; last observed free run number was 644 (rescan before use). Terrain remains default-off; C ABI exports unchanged.
Worktree intentionally detached. Sandbox rejected taskkill of own gate; original session interrupt successfully returned terminal exit 1.
Resume round r2: final-source qbuild all launched once; workspace and doc tests completed: 7484 passed, 0 failed (build-runs/qbuild-test.log). Clippy remains running; collect session 41130 to terminal result in this round. No game request outstanding.
Spec audit: section 15.5.1 step 5 requires direct view Init(terrain); section 15.5.3 publishes no Init slot. Native addresses are not a supported game interface. Do not guess a slot.
Spec audit: section 15.5.11 requires live terrain+0x6C and water+0x18 arrays; current plugin does not populate either. No replacement safety claim from the pointer correction alone.
Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-terrain-replace14-r2-manifest.json.
