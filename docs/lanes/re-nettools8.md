| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettools8: the network tool control, the intersection tool, the toll booth tool, the network preview and the placement-parameter copy, network-tool-owners.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-effects8) | **ready** (docs only). Five owners claimed: specified +11,948 bytes (+42 functions); network-tool-ui.md §3 corrected | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly, the decompile only as a guide; the key and type jump tables, the
intersection search offsets, the toll booth configuration table, the preview corner tables and the constants were read
in the PE. Written on 2026-10-25.

### What changed

- docs/specs/network-tool-owners.md (new): §1 the network tool control (state, Init/Shutdown, keys and buttons, the
  outline sized by the pen width, the directional cursor, the control-change message); §2 the intersection tool
  (the ring keys, the five-offset search over the rotation ring with dry placements, text, cost and funds, the rotated
  preview effect, placing and paying); §3 the toll booth preview tool (`PickLotAndState`, the footprint effect, the
  preview prop); §4 the network preview (records, corner order, heights, the two multimaps, the two-pass draw);
  §5 the `tPlacementParams` copy; §6 the forwarding correction; §7 quirks; §8 conformance tests.
- docs/specs/network-tool-ui.md §3: the UI front end's slots 4/5/6/14 forward to tool slots 42/43/45/44, not
  42/43/44/33 (corrected in place).

### Coverage

- tools/coverage-map.tsv: rows for the network tool control, the intersection tool and the toll booth tool before the
  `cSC4ViewInputControl*` catch-all, `tPlacementParams` before `cISC4*`; the `cSC4NetworkPreview` row filled.
- Specified bytes 5,081,362 → 5,093,310 (+11,948, 91.2%, `cov_local.py` with owners-c1).
