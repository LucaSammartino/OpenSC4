# renderer-driver: answered questions
<!-- Answered entries moved from ../renderer-driver.md. Rules: ../README.md. -->

<!-- from: re-driver4 (2026-10-21) -->
- **RD2 - sections 4.1 and 4.3:** the contents of `SetDefaultD3DDeviceState` (`0x008872A0`, 788 bytes): which
  render and texture-stage states it sets and to what; and the purpose of the word at object `+0x138` that `Init`
  resets to 7. [needs: a pass over `0x008872A0` with `re_query.py asm`]
  **ANSWERED 2026-10-21 (re-driverdraw4):** MATCHES — section 7: 26 render states, 4 driver flags, 12 stage-0/1
  texture-stage calls and an unbound stage-0 texture in fixed order (stencil depth-fail defaults to invert,
  texture factor 0xFFFFFFFF, lighting off, colour vertex on), then every stage record marked dirty; the `+0x138`
  word has no reader in 0x00881A00-0x0088B400 (write-only).
