---
tags: [core, openui-lang]
---

# Library

> The set of components a program may use.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Library`

## How it works

Declared as `pub struct Library`.

Positional arguments map to parameters by order.

Public API:

- `pub fn component(mut self, name: &'static str, params: &[(&'static str, Kind, bool)]) -> Self` — add a component.
- `pub fn signatures(&self) -> String` — one line per component, in the signature form the specification uses in prompts: `Stat(label: string, value: string, trend?: string)`.
- `pub fn new(root: Option<&'static str>) -> Self`
- `pub fn get(&self, name: &str) -> Option<&Component>`
- `pub fn names(&self) -> impl Iterator<Item = &'static str> + '_`

## Depends on

- [[Eval Module]] — uses `Component`, `Param`
- [[Kind]] — uses `Kind`

## Used by

- [[Eval Module]] — from `Evaluator`, `evaluate`
- [[cw-illustrate]] — from `library`

## See also

- [[_index]]
- [[Eval Module]]
