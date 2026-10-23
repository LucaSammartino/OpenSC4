# renderer-driver: answered questions
<!-- Answered entries moved from ../renderer-driver.md. Rules: ../README.md. -->

<!-- from: re-driver4 (2026-10-21); closed by re-gdriver5 -->
- **RD5 - section 1:** the texture-name, texture-upload and draw slots (24-29, 35-39, 51-62, 3-5, 68-73) and the
  `cTextureManager` / `cPrimitiveManager` contracts behind them (queueing, batch limits, when a queued batch is
  submitted besides state changes and `Flush`). Subsequent scope included slots 6-10, 40-42 and 67-76,
  stage records and the writers of +0x209/+0x263.
  **ANSWERED 2026-10-23 (re-gdriver5):** MATCHES — §§8–11 and §13 specify these interface contracts:
  queued versus immediate geometry, batch limits, stage caching, name/surface lifetime, upload ordering,
  matrices and unsupported blits. §12.3 indexes all graphics slots. +0x209 comes from mode capability byte
  +0x29 (§10.2); +0x263 is setup/teardown readiness (§9.5). Complete format ranking/converter arithmetic
  is tracked separately as RD6, not asserted by this interface answer.

<!-- from: re-driver4 (2026-10-21) -->
- **RD2 - sections 4.1 and 4.3:** the contents of `SetDefaultD3DDeviceState` (`0x008872A0`, 788 bytes): which
  render and texture-stage states it sets and to what; and the purpose of the word at object `+0x138` that `Init`
  resets to 7. [needs: a pass over `0x008872A0` with `re_query.py asm`]
  **ANSWERED 2026-10-21 (re-driverdraw4):** MATCHES — section 7: 26 render states, 4 driver flags, 12 stage-0/1
  texture-stage calls and an unbound stage-0 texture in fixed order (stencil depth-fail defaults to invert,
  texture factor 0xFFFFFFFF, lighting off, colour vertex on), then every stage record marked dirty; the `+0x138`
  word has no reader in 0x00881A00-0x0088B400 (write-only).
