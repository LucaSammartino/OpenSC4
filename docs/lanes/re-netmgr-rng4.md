| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netmgr-rng4 (answer NM12: capture point for the network-lot configuration draw) | `re-netmgr-rng4` / (RE spec lane) | ready. NM12 answered DIFFERS (in-process capture of generator `0x00B44870`, draw compared exactly, no resync); NM13 filed and answered DIFFERS (pending hash-set order, `B` from 193). network-manager.md §8.5, §15, §16 tests 25-26, §17.1-17.4. No engine/ABI edits; no game runs. | 1. Engine: NetworkLotRandom gains a raw-state load `(state[624], k)` and a post-state compare; shadow captures words at `0x00B44870`, `left` at `0x00B45234`, and `pending` B at mgr+0x78/+0x7C between operations. 2. Engine: replace ascending-key `pending` order with the §8.5 hash order. 3. Rerun vnmf4 turbo/normal expecting 0 draw resyncs. |

## Report

What changed (docs/specs/network-manager.md):
- §17.1 ownership: generator `0x00B44870` is a file-local static, seeded once at `0x00A73BF0`; only three draw
  sites in the whole executable (`0x0060D4AC` prop, `0x0060D512` flora, `0x0060DE59` configuration; the latter
  skipped when count = 0). No other instruction addresses the object range `0x00B44870`-`0x00B45237`. State is
  constant between network lot manager operations.
- §17.2 layout (byte-backed, ctor `0x00913B74`, draw `0x009139B9`, reload `0x009138A9`): `state[624]` +0, `next`
  +0x9C0, `left` +0x9C4; `k = 624 - left`. Exe base 0x400000, relocations stripped, DllCharacteristics 0.
- §17.3 capture/replay rule: in-process read sanctioned between operations; load as raw state, replay, then check
  (never copy) the post-state; an unexplained capture difference is reported, not absorbed. Answer to NM12(b): the
  draw is NOT outside exact comparison; resync after a draw-only difference is not specified.
- §17.4 worked vectors (S = 0x12345678, seed 0x9E3D9501; incl. a reload crossing at k = 622), computed from the
  foundations §6.1 algorithm by build-runs/mt_vec.py, not game output.
- §8.5 [compat]: NM13 answer - SGI hash_set at mgr +0x74 (buckets +0x78/+0x7C, count +0x84), bucket = key mod B,
  B starts 193 (ctor asks 100, primes at `0x00A829B0`), insert resizes with count+1 (`0x00710DD0`, head-push
  rehash), never shrinks; iteration ascending bucket, latest insert first. Worked example and §16 tests 25-26.
- Questions: NM12 and NM13 in questions/answered/network-manager.md; QUESTIONS.md regenerated.

Notes: REA was unavailable ("No app is open"); used re_query.py, instructions.asm and decomp/ instead.
Commits: 77efba0c (NM12), 3bb29349 (NM13), then tests and this report.
