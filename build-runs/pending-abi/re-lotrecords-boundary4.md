# LR9 public ABI correction proposal

Replace exactly slots 3 and 4 of vtable `00ab19b0` in each named ABI table using the two TSV files in this directory.
Keys, targets, argument counts and cleanup remain identical. Correct the class-specific method labels and certify
the pointer return in EAX. Do not rename these function addresses globally: shared thunks have other meanings
for other classes. No C ABI export, registration mechanism, layout change or hook is proposed.

Evidence: lot-records.md sections 3.1/13.2, constructor `006c2340`, QI `006bf190`, cast bytes
`006c2040`/`006c2050`, and native traffic-source iterator call `00710841` with AddRef `0071084e`.

Coordinator must review, apply and gate these proposals. They have not been installed in docs/abi.
