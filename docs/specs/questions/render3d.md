# render3d: open questions
<!-- Questions about [render3d.md](../render3d.md). Append new entries at the END; when one is answered, move it to answered/render3d.md. Rules: README.md. -->

- **R3D-1 - section 2:** the name of cIGZOStream/cIGZIStream slot 8 (`+0x20`) used for the viewport fields and the
  camera mode in `cSC43DRender::Write`/`Read` (`0x007C310B`, `0x007C31AD`): `PutSint32`/`GetSint32` is inferred from
  slot 9 being `PutUint32`. Settle from the stream vtable row in docs/abi/interfaces.tsv.
- **R3D-2 - section 1.3:** byte `+0x60` defaults to 1 and, per view3d.md section 4.1 step 12, suppresses `ClearViewport`
  in `Draw`; `GetBackgroundPicture` returns it as the "enabled" out flag. Which writer clears it (candidate:
  `SetBackgroundPicture` `0x007C2610`/`0x007C26D0`)? Until settled a replacement keeps default 1.
- **R3D-3 - section 5:** layout of the 0x30-byte `ModelChangeQueueItem` built by `0x007C2E50` (the bounding-box
  offsets read at item `+0x14`..`+0x2C` in `ProcessModelChangeQueue`) and the semantics of the `AddDirtyRectangle`
  mode argument (0/1 = class bit `0x200`, 2 = removal, 3 = kind 4, extra 8). Needs `0x007C2E50` and
  `AddDirtyRectangleInternal` `0x007C8F30`.
