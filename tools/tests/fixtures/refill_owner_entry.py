"""Copied into an owned fake root as tools/lane_refill.py by the supervision test.

Run the real dispatcher's main/lock/publication loop with launching mocked out.
"""
import json
import os
import sys
from unittest.mock import patch

sys.path.insert(0, os.environ['FC4_TESTED_TOOLS'])
import lane_refill
import lane_supervision

with patch.object(lane_refill.launcher, 'launch'), patch.object(lane_refill.launcher, 'alive', return_value=False):
    code = lane_refill.main()
if code:
    sys.exit(code)
print(json.dumps(dict(pid=os.getpid(), identity=lane_supervision.process_identity(os.getpid()))))
