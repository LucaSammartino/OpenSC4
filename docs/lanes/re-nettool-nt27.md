| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettool-nt27: scripted result contracts NT27–NT29 | `Codex` / `re-nettool-nt27` (RE, detached reserved worktree) | in progress | Trace public projections, failure 1 and retained cost inputs; answer questions and run doccheck |

## Handoff

- Goal: answer NT27–NT29 in network-tools with behaviour and conformance vectors.
- Read shadow5 report on its branch. It has no Owner closure table or `open:` rows; its Questions and next steps and three question entries are the available closure requests.
- Questions NT27–NT29 exist on glue-nettool-shadow5, but are absent from this base's open-question file. Preserve their exact text in the answered archive when closing.
- Existing corrected runs b/c both 5/14 exact; no new game run requested by this lane.
- Done: spec §§2.1/3.5/5.3/8.1.1/14 and vectors 20–30 written from export and local decomp.
- Key corrections: slot 14 Boolean, slot 23 coordinate-vector copy; diagonal rows contain turn cells; cell +0x52 is an intersection marker, not already-built.
- Proposed ABI corrections: build-runs/abi-rows-re-nettool-nt27.tsv (5 replacement rows); docs/abi untouched.
- Next: preserve original NT27–NT29 entries in answered archive, regenerate/check index, run doccheck and review citations/vectors.
- Main RE inputs: C:/Users/tlori/FreeCity4; edits only in this worktree.
