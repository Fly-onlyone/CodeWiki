---
tags: [core, openui-lang]
---

# Library

> The set of components a program may use.

## What it does

A `Library` lists the components an illustration may use. Each component has a name and an ordered list of parameters, and each parameter has a [[Kind]] and a required flag. The evaluator checks every component call against it. The same library also prints the component signatures that go into the model's prompt, so the model and the checker agree on one definition. Positional arguments map to parameters by order.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Library`

## How it works

`Library::new` takes an optional root component name. `component` adds one entry and returns the library, so calls chain. Entries sit in a sorted map, so `names` and `signatures` come out in alphabetical order.

`get` looks up a component by name; the evaluator calls it for each call in the program. `signatures` writes one line per component, such as `Stat(label: string, value: string, trend?: string)`. A required parameter has no `?`.

The root name is the fallback entry. When a program has no `root` statement, `evaluate` uses the first statement that calls this component. [[cw-illustrate]] builds the real library.

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

## Gotchas

- Parameters are positional. Their order in `component` is the order the model must write its arguments, and required ones must come first.
- Adding a component with an existing name replaces the old one.
- Names are `&'static str`, so a library can only be built from compiled-in names.

## See also

- [[_index]]
- [[Eval Module]]
