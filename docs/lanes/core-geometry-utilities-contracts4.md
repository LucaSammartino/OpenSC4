| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-geometry-utilities-contracts4: public geometry delta | Codex, `core-geometry-utilities-contracts4` | in progress | Implement GU4 raw primitives, signed clipping, bounded numerics; canonical gate |

## Handoff
- Goal: public geometry §§2–6.4,10.1,11; no ABI/plugin/game changes.
- Reserved detached worktree reused; initial tree clean, main.. empty.
- Task-specific instruction says no pre-edit baseline or redundant quick; run one stable committed-source all gate.
- Public 26-vector ledger recorded below before comparisons. Historical tests retained except documented public differences.
- Implemented raw GU4 primitive host, byte-NUL layout, common epilogue, format/plain/compressed helpers.
- Implemented signed clipping backing/count API and PC24 bounded length/plane/lighting/colour model.
- Updated builder local u16 count, dirty/reset and wrapped inline closure; foreign bounds algorithm unchanged.
- Added independent public vector tests. First focused qbuild quick is outstanding (transport handle 18).
- Next: fix focused failures, commit stable source, run the one canonical all gate and fmt check.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-geometry-utilities-contracts4-r1-manifest.json.
- No game requests. No agents. GU1/2/3/5/6 residuals remain open.

## Independent public vector ledger (before output comparison)
Expected values are copied from public §10.1; these are proposed, not native measurements.

| ID | Independent expected discriminator | Initial state |
|---|---|---|
| N1 | squared length 4B800001 | UNRUN |
| N2 | plane w BF800000 | UNRUN |
| B1 | tied extrema choose second signed zero | UNRUN |
| B2 | unordered extrema choose second; point NaN outside | UNRUN |
| B3 | touching false untouched; null minimum gates maximum | UNRUN |
| C1 | 41 initial copies, count 40, tail (40,0) | UNRUN |
| C2 | m<=0 preserves count 41 and all initial copies for any max | UNRUN |
| C3 | count min(40,max), nonpositive no copy-back | UNRUN |
| C4 | max=0 failed crossings do not read scratch | UNRUN |
| C5 | edge n<=0 count 0; polygon n=-1,m=0 count -1 | UNRUN |
| M1 | alpha 65537 converts to byte 128 | UNRUN |
| H1 | parallel off-plane false writes zero; box false untouched | UNRUN |
| H2 | null ray output chooses different algorithm | UNRUN |
| H3 | unordered triangle accepts, unordered hit | UNRUN |
| H4 | unordered segment parameter false untouched | UNRUN |
| V1 | unordered scale leaves input words | UNRUN |
| N3 | PC64 resident epsilon comparison differs from store | UNRUN / outside bounded PC24 work |
| M2 | u16 vertex wrap; no implicit closure | UNRUN |
| M3 | Reset preserves dirty; Begin appends without closure | UNRUN |
| T1 | GS absent: false; restore, size 32x16, rect 32,13,2,1, font release | UNRUN |
| T2 | upload false: mode3 through upload; common epilogue | UNRUN |
| U1 | base+36, 2x2, stride8; end before cleanup on false | UNRUN |
| U2 | AA*8,0000,FFFF,55*4; free before end on false | UNRUN |
| N4 | squared 4B800001, root 45800000 | UNRUN |
| M4 | channel1=255, negative-lit red=155 | UNRUN |
| M5 | first65535, final23, closure FFFF0018 | UNRUN |
