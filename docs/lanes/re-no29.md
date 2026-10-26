| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-no29 / network occupants questions (docs) | Codex / re-no29 / reserved branch re-no29, detached HEAD | ready (docs); commits 62e054b9f, 6f26bcd52, 5d76eb04a; ABI/seams unchanged; doccheck passed; diff check passed; game/shadow n/a; NO30/NO32 answered, NO40/UG5 narrowed | 1. Capture NO40-A/B/C before full geometry-owner credit. |

NO30 is answered in network-occupants.md §10.2 and moved to its answered archive.
Assembly 0061EE60–0061EE78 shows Clip runs before the option byte skips Bind and FlipAndLight.
Worked case distinguishes true/false option with local lower y=5 and retained alpha handling.
Questions index regenerated and committed; doccheck check and git diff --check pass.
A large patch and a read-only shell call stalled and were cancelled; tools then recovered.
The failed large patch did not land. Smaller edits were verified and committed afterward.
No REA was used. Sources are main checkout export, class decomp files and bin/SimCity4.exe.
NO32 evidence: AA7D98 U=(0,0,1,1), AA7D88 V=(0,1,1,0), all f32.
B0B374 is one u32 cached zoom (initial 5), copied directly from context+14 by 0061C5F0.
GetTexture 0061C950 adds it iff low nibble of texture ID is zero; there is no zoom array.
Draw 0061A9B3–0061A9BD uses primary slot 9 (HasAny), so either 06000000 bit enters.
StaticInit A/B/C nearest-even packed value/mask pairs: 024A91FF/1FFFFFFF,
020001E7/1E0003FF, 0242317F/1FFFFFFF; published with address citations and worked cases.
UpdateLighting 0061BC57–0061BCBB normalizes with no zero guard and calls lighting slot 19.
It copies returned RGB to all triangle vertices as B,G,R at +12,+13,+14, preserving alpha.
Degenerate zero normal produces NaNs with masked exceptions; no owner fallback colour.
Lighting §4.1 and §4.7 specify query and byte conversion; 007D7840 confirms it.
Temporary PE reader and commit message file are in build-runs/.
NO40/UG5 publish custom kernels, ratio indexing, walls/UVs, bounds, connection descriptors and Draw.
Remaining: selector arguments, base transform composition, exact connection matrices/ceiling/input bytes.
Runtime captures NO40-A/B (§3.2.3) and C (§3.6.6) name sites, lengths, variants and settled expectations.
No full geometry-owner closure is claimed. No ABI rows proposed; engine untouched; game files read only.
Next: run those captures and reconcile with the independent algorithms before geometry-owner credit.
