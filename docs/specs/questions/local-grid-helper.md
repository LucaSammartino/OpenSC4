# Local grid helper questions

- **LGH1 — §2/§3.2/§5, scalar declaration and ambient FP environment:** which
  declared return precision does the `0x00B43CF4` interface's `+0x18` accessor
  guarantee, and which x87 PC/RC, exception masks, MXCSR and floor dispatch
  settings are guaranteed on all valid tool-entry threads? Helper instructions
  establish ST(0) consumption, five separate calls, binary64/binary32 stores
  and no local control-word setup. They do not establish those startup/foreign
  contracts. The spec's exact literal vectors require their explicit finite
  primitive replies/environment; arbitrary-bit native equivalence remains
  unclaimed. Do not widen this into region/view/terrain algorithms.
