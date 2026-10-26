# cSC4WinSnapshotTaker

Existing portable workflow, reused without a native binding in this lane.

| snapshots.md section | Code | Spec fixture / test | Explicit open boundary |
|---|---|---|---|
| 2 Creation/input | `SnapshotTaker` creation, size/key/mouse methods | `taker_size_digit_and_four_delivery_fixture`: last Space wraps, digit 5 gives 60 | SN1 percentage meaning; SN6 supplied sizes/portable positioning; SN7 native toolkit |
| 2 Capture | `deliver_capture`, image/album pipeline | `capture_pipeline_depth_metadata_pause_and_restore`; `failed_snapshot_write_removes_partial_png_and_reports_cleanup_error` | SN8 image pixel masks/codecs and failure behavior; SN9 filesystem failures |
| 6 Conformance | `taker_size_digit_and_four_delivery_fixture` | Worked cases 4-5: size wrap/60 percent; counter 0 captures on fourth delivery | No exact game owner replacement claim |

The parent README covers the full spec and open SN1-SN9 questions. Capture
already calls `SnapshotAlbum` storage methods through primitive file/image/UI
traits; this lane adds no C ABI or plugin path.
