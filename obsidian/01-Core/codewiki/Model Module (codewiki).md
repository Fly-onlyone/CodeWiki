---
tags: [core, codewiki, module]
---

# Model Module (codewiki)

> `codewiki model`: the built-in embedding models from the command line.

## Source

- `crates/codewiki/src/model.rs` — module `codewiki::model`

## How it works

Main items:

- [[ModelAction]] — enum
- `run` — function
- `MB` — const
- `store` — function
- `entry` — function
- `megabytes` — function
- `install` — function
- `list` — function

## Depends on

- [[Store]] — uses `Store`, `install`, `is_installed`, `new`
- [[Settings Module (cw-core)]] — uses `EMBED_MODEL`, `EMBED_PROVIDER`, `process_env`
- [[Catalog Module]] — uses `all`, `find`
- [[Entry]] — uses `Entry`, `size`
- [[cw-models]] — uses `ENGINE`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[codewiki]]
