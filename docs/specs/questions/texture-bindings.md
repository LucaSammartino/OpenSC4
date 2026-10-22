# texture-bindings: open questions
<!-- Questions about [texture-bindings.md](../texture-bindings.md). Append new entries at the END; when one is answered, move it to answered/texture-bindings.md. Rules: README.md. -->

<!-- from: re-texbind (2026-10-19) -->
- **TB1 - §1.1:** which interface is `0x088CEB67`, answered at `+0x04` of the binding factory? Its vtable
  (`0x00ABD8B8`) has the three `this - 4` thunks and one shared slot that returns at once.

<!-- from: re-texbind (2026-10-19) -->
- **TB2 - §1.9:** who sets the factory's display bit depth (slot 10) and its LOD offset (slot 9), and from which
  settings? graphics-formats.md §2.10 infers the texture-quality option for the LOD offset.

<!-- from: core-textures-materials4 (2026-10-22) -->
- **TB4 - §§2–3 complete owner/body inventory:** publish the missing `cS3DMaterial` methods and lifetime (including whether/how its 16-bit count and held texture bindings are released), and material-manager primary slots 6/7 (`0x0062C980` in the public ABI table). Section 3 publishes the material constructor state only and manager slots 3–5, not these bodies. Also reconcile any owner-private/destructor methods absent from §§1–3 against the requested 58-function owner inventory. [Core implements the published constructor state and manager slots 0–5 with explicit foreign system-service primitives; no material reference/binding-release or slots 6/7 contract is invented, and no blanket four-owner completion claim is made.]

  **Partial proof 2026-10-22 (re-small-renderer-contracts4):** §§3–3.1 establish slots 6/7, manager destruction, material release at counts 0/1, wrapping acquisition, reverse binding destruction, and value-copy semantics. Residual: legacy 58/4883 counting scope versus 55 named bodies/4758B plus four class/create helpers/111B; no blanket inventory-completion claim.
