# Native lot configuration editor questions

## LE1 — Compatibility multi-selection maps and first scalar write-back

In [lot-configuration-editor.md](../lot-configuration-editor.md) §4, reconcile
all rows of the three multi-select controls created by `0x00480D10`, their
selected-index-to-model-bit mapping and optional-control behavior. Also trace
the `0x294053DB` scalar write-back. The decompiler loses stack tracking after
the early control queries; native instructions must establish these mappings.
Known: modal acceptance is `0x42B7C356`, scalar writes and checkbox byte rules
are already documented. No complete-owner credit until these and the exact
remaining body inventory are resolved.
