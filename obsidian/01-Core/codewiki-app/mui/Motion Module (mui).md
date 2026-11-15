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
- [[Enter]] — an entrance: where the element comes from, on which spring, after what wait.
- `scale` — the factor every motion is stretched by; 1 unless `CODEWIKI_MOTION_SCALE` says otherwise.
- `EnterExt` — elements that can make an entrance.
- `scaled` — `duration` stretched by `scale`.
- `STAGGER_CAP` — items after this many enter together with the last staggered one.
- `stagger_delay` — how long item `index` of a staggered group waits before it enters.
- `SNAPPY` — `spring.snappy`: the default for things that enter or follow the pointer.

## Depends on

- [[Enter]] — uses `Enter`, `at`
- [[Spring]] — uses `Spring`

## Used by

- [[codewiki-app]] — from `check_for_update`, `connect`, `install_update`, `new`
- [[Doc View Module]] — from `draw_strip`, `line`, `render`, `stat_tile`
- [[Ripple Module]] — from `finished`, `frame`, `moving`
- [[Spring]] — from `animation`, `config`, `policy`
- [[Button Module]] — from `ease`, `render`
- [[Enter]] — from `card`, `row`
- [[Illustration Module]] — from `tabs`
- [[Feedback Module]] — from `linear_progress`

## See also

- [[_index]]
- [[codewiki-app]]
