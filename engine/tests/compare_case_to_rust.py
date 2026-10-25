# SPDX-License-Identifier: GPL-3.0-or-later
"""Convert value-only compare JSONL into an includable Rust vector module."""
import argparse
import json
import re
from pathlib import Path


def integer(value, maximum=0xffffffff):
    if type(value) is not int or not 0 <= value <= maximum:
        raise ValueError("invalid unsigned value")
    return str(value)


def array(values, maximum=0xffffffff):
    return "&[" + ",".join(integer(v, maximum) for v in values) + "]"


def windows(values):
    parts = []
    for value in values:
        parts.append("Window { base: " + integer(value["base"], 16)
                     + ", offset: " + integer(value["offset"], 65536)
                     + ", bytes: " + array(value["bytes"], 255)
                     + ", image_offsets: " + array(value["image_offsets"]) + " }")
        for at in value["image_offsets"]:
            if at + 4 > len(value["bytes"]):
                raise ValueError("image offset outside window")
            if int.from_bytes(bytes(value["bytes"][at:at + 4]), "little") >= 0x7c8000:
                raise ValueError("image RVA outside supported module")
    return "&[" + ",".join(parts) + "]"


def convert(path):
    lines = ["// SPDX-License-Identifier: GPL-3.0-or-later",
             "// Generated value-only compare cases. Image pointer words are RVAs, never VAs.",
             "#[derive(Debug)]",
             "pub struct Window { pub base: u32, pub offset: u32, pub bytes: &'static [u8], pub image_offsets: &'static [u32] }",
             "#[derive(Debug)]",
             "pub struct Case { pub name: &'static str, pub source: &'static str, pub mutation: bool, pub rva: u32, pub contract: u32, pub args: &'static [u32], pub input: &'static [Window], pub native: &'static [Window], pub ours: &'static [Window], pub native_return: u32, pub ours_return: u32 }",
             "pub const CASES: &[Case] = &["]
    for raw in path.read_text(encoding="utf-8").splitlines():
        case = json.loads(raw)
        if case["version"] != 1 or not re.fullmatch(r"[A-Za-z0-9_-]+", case["name"]):
            raise ValueError("unsupported version/name")
        if case["source"] not in ("natural", "fixture", "corpus", "dense") or type(case["mutation"]) is not bool:
            raise ValueError("invalid source/mutation")
        fields = ["name: " + json.dumps(case["name"]), "source: " + json.dumps(case["source"]),
                  "mutation: " + str(case["mutation"]).lower(),
                  "rva: " + integer(case["rva"], 0x7c7fff), "contract: " + integer(case["contract"]),
                  "args: " + array(case["args"])]
        fields += [key + ": " + windows(case[key]) for key in ("input", "native", "ours")]
        fields += [key + ": " + integer(case[key]) for key in ("native_return", "ours_return")]
        lines.append("    Case { " + ", ".join(fields) + " },")
    return "\n".join(lines + ["];", ""])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case_file", type=Path)
    parser.add_argument("--output", type=Path, help="engine/tests/vectors/<name>.rs (default: stdout)")
    args = parser.parse_args()
    text = convert(args.case_file)
    if args.output:
        expected = Path(__file__).resolve().parent / "vectors"
        output = args.output.resolve()
        if output.parent != expected or output.suffix != ".rs":
            parser.error("output must be a .rs file directly under engine/tests/vectors")
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")
