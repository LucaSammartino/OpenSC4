# render3d: answered questions (archive)
<!-- Archive for [render3d.md](../../render3d.md). Lanes do not read this file. Open questions are in ../render3d.md. -->

- **RND3 - section 5:** layout of the 0x30-byte `ModelChangeQueueItem` built by `0x007C2E50` (the bounding-box
  offsets read at item `+0x14`..`+0x2C` in `ProcessModelChangeQueue`) and the semantics of the `AddDirtyRectangle`
  mode argument (0/1 = class bit `0x200`, 2 = removal, 3 = kind 4, extra 8). Needs `0x007C2E50` and
  `AddDirtyRectangleInternal` `0x007C8F30`.
  **ANSWERED 2026-10-21 (re-models4):** MATCHES — item layout is model +0, kind byte +4, unused byte +5, unused 16 bytes +8, box min +0x18 / max +0x24 (6 floats); modes 0 model draw, 1 class bit 0x200, 2 removal, 3 kind-4 change, 6 merged; merge scoring is nodeArea+newArea-unionArea minus min(areas) when either mode is 0, thresholds 16 nodes / -2500.0 (render3d.md section 5 and 6.1).
