"""Compare the reference parser's AST dump with the Rust parser's.

Usage: python compare.py reference.json rust.json corpus.txt

The `fn` flag the reference sets on custom function calls is ignored: the Rust
AST records the `@` prefix on every function call instead.
"""
import json
import sys


def normalize(node):
    if isinstance(node, dict):
        return {k: normalize(v) for k, v in node.items() if k != "fn"}
    if isinstance(node, list):
        return [normalize(v) for v in node]
    return node


reference = normalize(json.load(open(sys.argv[1], encoding="utf-8")))
rust = normalize(json.load(open(sys.argv[2], encoding="utf-8")))
programs = open(sys.argv[3], encoding="utf-8").read().replace("\r\n", "\n").split("\n-----\n")

if len(reference) != len(rust):
    sys.exit(f"program count differs: reference {len(reference)}, rust {len(rust)}")
failures = 0
for index, (a, b) in enumerate(zip(reference, rust)):
    if a != b:
        failures += 1
        print(f"--- program {index + 1} differs ---")
        print(programs[index])
        print("reference:", json.dumps(a, ensure_ascii=False))
        print("rust:     ", json.dumps(b, ensure_ascii=False))
print(f"{len(reference) - failures}/{len(reference)} programs match")
sys.exit(1 if failures else 0)
