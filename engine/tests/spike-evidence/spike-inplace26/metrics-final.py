"""Recompute the final S3 rates from actual first/evidence commit timestamps."""
import json
from datetime import datetime
from pathlib import Path

data = json.loads(Path(__file__).with_suffix(".json").read_text())
parse = lambda key: datetime.fromisoformat(data[key].replace("Z", "+00:00"))
seconds = (parse("end_utc") - parse("start_utc")).total_seconds()
assert seconds == data["elapsed_seconds"]
assert (parse("end_utc") - parse("base_utc")).total_seconds() == data["elapsed_from_base_seconds"]
print(f"lane_seconds={seconds:.0f} lane_hours={seconds / 3600:.6f}")
for key in ("qualified_body_bytes", "observed_no_outside_body_bytes", "selected_body_bytes"):
    rate = data[key] / 1000 / (seconds / 3600)
    print(f"{key}={data[key]} KB/hour={rate:.6f}")
print(f"baseline={data['baseline_kb_per_hour']} target={data['pass_kb_per_hour']} KB/hour")
assert data["detected_events"] == data["mutation_events"] == 33
assert data["outside_signal_functions"] <= 6 and data["float_only_functions"] == 0
assert data["selected_body_bytes"] / 1000 / (seconds / 3600) < data["pass_kb_per_hour"]
