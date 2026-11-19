"""Reference embeddings from sentence-transformers, to compare candle with.

    uv run --no-project --with sentence-transformers --with torch \
        --index https://download.pytorch.org/whl/cpu --index-strategy unsafe-best-match \
        python reference.py models/<name> texts.json <out.json>

Writes the same shape as `spike embed`: unit vectors for queries and documents.
"""

import json
import sys

from sentence_transformers import SentenceTransformer

model_dir, texts_path, out_path = sys.argv[1:4]
texts = json.load(open(texts_path, encoding="utf-8"))
model = SentenceTransformer(model_dir, device="cpu")
# The spike loads half-precision weights as f32; do the same here.
model = model.float()
prompt = "query" if "query" in (model.prompts or {}) else None
queries = model.encode(texts["queries"], prompt_name=prompt, normalize_embeddings=True)
documents = model.encode(texts["documents"], normalize_embeddings=True)
json.dump(
    {"queries": queries.tolist(), "documents": documents.tolist()},
    open(out_path, "w", encoding="utf-8"),
)
print((queries @ documents.T).round(4).tolist())
