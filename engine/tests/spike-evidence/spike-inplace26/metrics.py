"""Recompute S3 byte rates from the recorded lane commit timestamps (decimal KB)."""
import json
from datetime import datetime
from pathlib import Path

data = json.loads(Path(__file__).with_suffix(".json").read_text())
parse = lambda key: datetime.fromisoformat(data[key].replace("Z", "+00:00"))
seconds = (parse("end_utc") - parse("start_utc")).total_seconds()
base_seconds = (parse("end_utc") - parse("base_utc")).total_seconds()
assert seconds == data["elapsed_seconds"]
assert base_seconds == data["elapsed_from_base_seconds"]
print(f"lane_seconds={seconds:.0f} lane_hours={seconds / 3600:.6f}")
print(f"base_seconds={base_seconds:.0f} base_hours={base_seconds / 3600:.6f}")
for name in ("verified_body_bytes", "provisional_exact_confined_body_bytes", "selected_body_bytes"):
    print(f"{name}={data[name]} KB/hour={data[name] / 1000 / (seconds / 3600):.6f}")
print(f"PASS target={data['pass_kb_per_hour']} KB/hour; baseline={data['baseline_kb_per_hour']} KB/hour")
