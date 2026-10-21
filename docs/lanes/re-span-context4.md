| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-span-context4 (RenderToBuffer draw-context ABI for types 4/6/7; SP5, SR1555-7) | workflow / `re-span-context4` (spec lane) | **ready** (spec only): span-render-4444.md §1.3 full context and buffer ABI plus fixture route; span-render-1555.md §1.3; SP5 and SR1555-7 answered MATCHES. No engine, tools or ABI edits | 1. Glue lanes: drive slot 22 through a COM-created context per 4444 §1.3 steps 1-5 and report constructed (native context), constructed (direct) and shipped counts separately. 2. Reconcile build-runs/abi-rows-re-span-context4.tsv into docs/abi |

## Report

**Method.** REA had no binary open ("No app is open"), so all evidence is static: raw vtable dwords read from
bin/SimCity4.exe (build-runs/dw.py), `RET` immediates and instructions from `re_query.py asm`, and decomp/ for
orientation only.

**What changed** (commit d5712719):
- `docs/specs/span-render-4444.md` §1.3 (new). How to get each object: buffer CLSID `0xC470D325`, GetSystemService
  service `0xEB2EA3CB` / IID `0x4B2EA3B3` (argument order read at `0x0082C8CC`-`0x0082C8D8`), draw system slot 4.
  Factory order: RenderToBuffer first; SimGL needs `0xE3094DC8`, which cGZBuffer does not answer. Slot 4 overwrites
  `*out` without releasing the old value.
  - cIGZBuffer slot table: Init slot 3 `(w, h, type, bits)` pops 0x10, Lock/Unlock, pack, base, stride. Stride rule
    with examples.
  - Lock/Unlock audit. Every context lock is paired with an unlock that uses the same flags (`0x8040` for the
    destination, `0x800` for the texture). Unlock does not check its flags. The final Release does not unlock.
  - Full 24-slot context vtable `0x00AC17D0`, with arguments, pops and field effects. Lists which fields the
    constructor and Init leave indeterminate (`+0x40`, `+0x42`, gradient, colours) and what each path must set first.
  - Pack-colour rule for each type, with a worked example.
  - Five-step original-versus-ours fixture route, and the labelling rule for counts: constructed (native context),
    constructed (direct) and shipped.
- `docs/specs/span-render-1555.md` §1.3: SR1555-7 (a)-(d) answered, pointing to 4444 §1.3.
- Questions: SP5 and SR1555-7 moved to answered (MATCHES). QUESTIONS.md reindexed.

**Proposed ABI rows:** `build-runs/abi-rows-re-span-context4.tsv` (17 rows: context slots 3, 4, 7-9, 12-15, 17-19, 22;
cGZBuffer 3, 6, 7; draw system 4). Not applied to docs/abi.

**Note for the widget-toolkit owner:** widget-toolkit.md around line 704 calls `0x4B2EA3B3` the system service ID.
The call bytes show it is the IID; the service ID is `0xEB2EA3CB`. This lane did not edit that spec.

**Not done:** no runtime counts (spec lane: no engine edits, no game runs). Shipped producers of types 4/6/7 are
still open as SP4 and SR1555-6.
