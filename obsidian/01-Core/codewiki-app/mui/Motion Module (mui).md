---
tags: [core, codewiki-app, module]
---

# Motion Module (mui)

> How things move: springs, entrances and stagger.

## Source

- `crates/codewiki-app/src/mui/motion.rs` — module `codewiki_app::mui::motion`

## How it works

Main items:

- [[Spring]] — A spring as Framer Motion describes one.
- `Enter` — an entrance: where the element comes from, on which spring, after what wait.
- `scale` — the factor every motion is stretched by; 1 unless `CODEWIKI_MOTION_SCALE` says otherwise.
- `EnterExt` — elements that can make an entrance.
- `scaled` — `duration` stretched by `scale`.
- `STAGGER_CAP` — items after this many enter together with the last staggered one.
- `stagger_delay` — how long item `index` of a staggered group waits before it enters.
- `SNAPPY` — `spring.snappy`: the default for things that enter or follow the pointer.

## Depends on

- [[Curve Module]] — uses `EASE_OUT`, `SHARP`
- [[Duration Module]] — uses `ENTERING_SCREEN`, `LEAVING_SCREEN`
- [[Tokens Module]] — uses `easing`
- [[Spring]] — uses `Spring`

## Used by

- [[codewiki-app]] — from `check_for_update`, `connect`, `draw_queued_diagrams`, `forget_saved_note`
- [[Doc View Module]] — from `column`, `draw_strip`, `line`, `render`
- [[Ripple Module]] — from `finished`, `frame`, `moving`
- [[Spring]] — from `animation`, `config`, `policy`
- [[Button Module]] — from `ease`, `render`
- [[Illustration Module]] — from `tabs`
- [[Feedback Module]] — from `linear_progress`
- [[List Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
