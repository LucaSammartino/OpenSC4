tag: split11-initial-622
run: 622
date: 2026-10-24
lane: tool-regress-split11
dll: main ffb8f38c2 engine, rebuilt by qbuild verify; sha256 0a67d38e6a8c1fbf969f7e483132fa17ead29076f3a0d248d8ddff2037ede82c
pacing: turbo to 2032-01-01, queue, initial ShadowSplit 3 implementation
verdict: FAIL; invalid grouping, not a successful split measurement
switches: house shadows and standard replacements
result: regress-split11-s3 wall 868 s. Real execution exposed Sort-Object property lookup on PowerShell hashtables: groups 1/2 had no shadows and stalled on shutdown; group 3 had the full house profile. The regression correctly failed despite the last group being clean. Fixed in c46aa442c using explicit sort expressions, with a balance assertion added to the real PowerShell test. These results are not combined or corrected-source evidence.
summary: Failed initial attempt retained separately; no coverage credit claimed.
