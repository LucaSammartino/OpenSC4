tag: run-37
run: 37
date: 2026-10-13
lane: run
dll: unknown
pacing: unknown
verdict: clean
switches: same DLL, `enable = plumbing, flammability`, FC4 Test's Konradshohe, 240 s
result: **clean verdict but `ref_warnings=4`: a real use-after-free.** Flammability works: 4 fire rolls and 4 end-of-cycle passes, roughly every 40 days, and 22,824 `IsCellWatered` queries into plumbing (7,258 watered, 0 "no plumbing"), so the cross-subsystem read is live. On the first real ignition (`ignited=1/1`, 2026-04-18), `IgniteCell`, `Drop` and `SetFireCause` released references to the game's fire object that we never acquired (2 over-releases, 2 last-reference releases), so the object is freed while the game still owns it. Also `reentrant` went 0 → 1 after the fire, and the flammability `average` read 0 for the first two months after load. **Fixed in `95db886`**: the plugin was releasing the *borrowed* disaster manager in both `IgniteCell` and `SetFireCause` (spec §6.8 releases only the QueryInterface'd fire interface). The average of 0 is spec behaviour (§14.1 quirk 1: the cycle position survives `Init`, so the first end-of-cycle after a load is empty).
