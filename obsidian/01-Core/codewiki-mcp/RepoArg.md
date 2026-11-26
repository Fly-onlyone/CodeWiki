---
tags: [core, codewiki-mcp]
---

# RepoArg

> A tool's `repo` argument: a folder path or a repository id or name.

## What it does

Many MCP tools take a `repo` argument. An agent may write a folder path, a repository id, or a repository name there. `RepoArg` keeps which of the two it was, so the guard can treat paths and names differently. It has two cases: `Path` holds an absolute folder, and `IdOrName` holds a bare word.

## Source

- `crates/codewiki-mcp/src/guard.rs` — definition of `RepoArg`

## How it works

`repo_arg` in [[Guard Module]] builds it. A value is a `Path` when it is absolute, starts with a drive letter, starts with a dot, or contains a slash or backslash. A path is made absolute against the working directory. Anything else is an `IdOrName`.

`lookup_name` turns it into a name for the repository list. `target` turns it into what the backend receives: a repository id, or an absolute path. `direct_path` makes a folder out of it to test against the closed list.

## Items

Declared as `pub enum RepoArg`.

## Used by

- [[Guard Module]] — from `direct_path`, `lookup_name`, `repo_arg`, `target`

## Gotchas

- A bare word that no repository answers to is still a folder, under the working directory. It is never passed on as a word.
- A value with a drive but no root, such as `D:secret`, is a `Path`. This stops it passing as a name and slipping past the closed folder list.
- A path is stored already absolute, so compare it with the closed list as it is.

## See also

- [[_index]]
- [[Guard Module]]
