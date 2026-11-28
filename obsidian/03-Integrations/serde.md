---
tags: [integration]
---

# serde

> External crate `serde`, imported by 34 source files.

## What it does

serde turns Rust structs and enums into data formats and back. In CodeWiki the format is almost always JSON through `serde_json`. It shapes the HTTP API bodies, the MCP tool parameters, the manifest and list files kept on disk, the facts stored in the index, and the pictures an agent sends. A type that is sent or saved derives `Serialize`, `Deserialize`, or both.

## Used for

Imported by 34 source files. Main users:

- [[Agents Off Module]] — imports `Deserialize`, `Serialize`
- [[Api Module]] — imports `Deserialize`, `Serialize`
- [[Cdeps Module]] — imports `Deserialize`
- [[Ci Module]] — imports `Deserialize`, `Serialize`
- [[Contracts Module]] — imports `Deserialize`, `Serialize`
- [[Coverage Module]] — imports `Deserialize`, `Serialize`
- [[Decisions Module]] — imports `Deserialize`, `Serialize`
- [[Embed Job Module]] — imports `Serialize`

## Configuration

- `crates/codewiki-mcp/Cargo.toml`, `crates/cw-illustrate/Cargo.toml`, `crates/cw-server/Cargo.toml`, and 3 more — declare the dependency: version `1`, locked `1.0.229`, features `derive`
- `crates/cw-core/Cargo.toml`, `crates/cw-index/Cargo.toml`, `crates/cw-parse/Cargo.toml` — declare the dependency: version `1.0.229`, locked `1.0.229`, features `derive`

## Wire-up

Crates enable the `derive` feature and write `#[derive(Serialize, Deserialize)]` on a type. No registration is needed.

Common attributes in the code:

- `rename_all` sets how names look on the wire. `Language` in `cw-core` uses lowercase. Most other enums there, such as symbol and edge kinds, use snake_case.
- `default` fills a missing field. Request bodies and query structs in the API use it, so a client may leave a field out. `default = "..."` names a function, as for the search limit.
- `skip_serializing_if` leaves out an empty field, such as an empty list, or an unset optional field of a repository entry in the list file.
- `flatten` merges a nested struct into its parent object.

In the MCP bridge, parameter structs derive `Deserialize` next to `schemars::JsonSchema`. The doc comment on each field becomes the tool's field description, and the many optional parameters carry `default`. The `agents_off` module saves its folder list as a small JSON document with a `version` field.

## Auth mode

No sign-in. serde holds no secret and talks to no service.

## Gotchas

- Adding a field without `#[serde(default)]` to a type that is read from disk breaks older files. Stored data such as symbol `attrs` and the optional fields of a registered repository use `default` for this reason.
- A renamed enum variant changes the stored or sent text. `rename_all` is part of the format, so change it with care.
- Files with a `version` field, such as the agents-off list, refuse a version they do not know instead of guessing.
- On the MCP side, a doc comment on a parameter field is the text an agent reads. Editing it changes the tool's description.
- An unknown field in a request body is ignored unless the type says otherwise, so a misspelled option fails silently.

## See also

- [[_index]]
