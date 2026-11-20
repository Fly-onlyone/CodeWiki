---
tags: [core, openui-lang]
---

# Library

> The set of components a program may use.

## What it does

Positional arguments map to parameters by order.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Library`

## How it works

1. `library` in [[cw-illustrate]] calls `new`.

## Items

Declared as `pub struct Library`.

Members:

- `pub fn new(root: Option<&'static str>) -> Self` — public method, returns `Self`; called by `library`.
- `pub fn component(mut self, name: &'static str, params: &[(&'static str, Kind, bool)]) -> Self` — add a component. `params` lists `(name, kind, required)`.
- `pub fn get(&self, name: &str) -> Option<&Component>` — public method, returns `Option<&Component>`.
- `pub fn names(&self) -> impl Iterator<Item = &'static str> + '_` — public method, returns `impl Iterator<Item = &'static str> + '_`.
- `pub fn signatures(&self) -> String` — one line per component, in the signature form the specification uses in prompts: `Stat(label: string, value: string, trend?: string)`.

## Depends on

- [[Eval Module]] — uses `Component`, `Param`
- [[Kind]] — uses `Kind`

## Used by

- [[Eval Module]] — from `Evaluator`, `evaluate`
- [[cw-illustrate]] — from `library`

## See also

- [[_index]]
- [[Eval Module]]
