tag: run-88
run: 88
date: 2026-10-14
lane: run
dll: d3ca1ef3
pacing: unknown
verdict: clean
switches: main `ac159da` (DLL `d3ca1ef3`), `build-runs/save-res.ps1`: fourteen replacements, (a) a real Ctrl+S save 90 s after load (`-KeepSaves`), (b) a reload of that save for 90 s, then the test city restored from `city-backups` (hash 864E0A88 checked)
result: **Save → reload works with our residential.** (a) `residential Write -> 1`, file written (the harness did not see the confirmation dialog, but the file changed); (b) `residential Read -> 1` at the saved 2030-11-29 with population 24,293, and Init restores R$ 22,314 / R$$ 1,728 / R$$$ 251, EQ 39.248, HQ 44.361; the city runs on to 2035. Both verdicts clean, **but (a) has `ref_warnings=3`**: at Shutdown, after the save, the land value glue's own `ReleaseGrid` (`cFC4LandValue.cpp`) released the last reference to its three grids from `Read` (intrinsic, total, slope; classes 0x49B9E602 ×2 and 0x49B9E60A). Runs without a save never warn, so the save presumably drops a reference the game held since the load; land value releases with `FC4_RELEASE` where the other glues treat a grid from the save as created for us. Fix lane: land value grid references.
