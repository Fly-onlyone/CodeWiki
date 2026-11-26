---
tags: [core, openui-lang]
---

# Kind

> `Kind` enum defined in `openui_lang::eval`.

## What it does

`Kind` names the type of one component parameter: `Text`, `Number`, `Bool`, `List`, `Element` or `Any`. A [[Library]] stores a `Kind` for each parameter, and the evaluator checks every argument against it. The same value also decides the type word printed in prompt signatures.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Kind`

## How it works

When a component call is evaluated, each argument value goes through `matches_kind` with the parameter's `Kind`. A mismatch gives a `type-mismatch` error, and a required parameter then drops the whole component.

`Library::signatures` maps each `Kind` to a word for the prompt: `Text` is `string`, `Number` is `number`, `Bool` is `boolean`, `List` is `array`, `Element` is `component`, `Any` is `any`.

## Items

Declared as `pub enum Kind`.

## Used by

- [[Eval Module]] — from `Param`, `matches_kind`
- [[Library]] — from `component`, `signatures`

## Gotchas

- `Text` also accepts a number. A numeric value passes where a string is expected.
- `List` accepts a list of anything; it does not check the items.
- `Element` accepts only one component. A reference that resolves to nothing is `Null`, and `Null` never reaches this check: it counts as missing.

## See also

- [[_index]]
- [[Eval Module]]
