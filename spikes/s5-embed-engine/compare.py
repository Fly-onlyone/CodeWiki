"""Compare two files of unit vectors: the largest difference between the
query-document similarities, and the lowest cosine between matching vectors.

    python compare.py reference.json candle.json
"""

import json
import sys


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


reference, candle = (json.load(open(path, encoding="utf-8")) for path in sys.argv[1:3])
lowest = 1.0
for part in ("queries", "documents"):
    for a, b in zip(reference[part], candle[part], strict=True):
        lowest = min(lowest, dot(a, b))
largest = 0.0
for rq, cq in zip(reference["queries"], candle["queries"]):
    for rd, cd in zip(reference["documents"], candle["documents"]):
        largest = max(largest, abs(dot(rq, rd) - dot(cq, cd)))
print(f"lowest cosine between matching vectors: {lowest:.6f}")
print(f"largest difference of a similarity:     {largest:.6f}")
