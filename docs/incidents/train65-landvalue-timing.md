# Train65 land-value comparison gap

The train65 gate and both2032/ref0 game verdicts pass, but the land-value comparison is71/72 exact (train64:72/72). Coverage correctly drops verified16.4% to16.3%.

At2029-07-07 pass43:5/4096 intrinsic cells differ (max3),2/4096 total cells differ (max3),1/256 slope cells differs (max0.7350807189941406), and0/4096 types differ. Mean altitude is270.05667 native versus270.05804 ours. Native dirty/running state is set (`settled=0`). The diagnostic finds native intrinsic/mode/mean matching the first of30 SimNewDay snapshots since the last pass; that snapshot still differs in376 total cells. This supports investigating read timing; it does not prove every cause.

LV1 already establishes per-Step reads and the previous developer maps. LV2 asks for the runtime comparison of those inputs. `verify-landvalue-step4` was interrupted by the provider weekly limit before implementing capture; preserve its worktree and resume only after fresh guards and provider availability. No algorithms, sampling or expected values were changed.

Evidence: `build-runs/run4-failures/train65-landvalue-timing/` retains the original log and verdict. The failing trace is `landvalue shadow2029-07-07 pass43` in regress-train65-shadows.log (the original line contains spaces after shadow).
