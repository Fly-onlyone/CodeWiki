---
tags: [core, codewiki-app, module]
---

# Themes Module

> Bundled color themes and theme switching.

## Source

- `crates/codewiki-app/src/themes.rs` — module `codewiki_app::themes`

## How it works

Main items:

- `list` — every loaded theme as `(name, is dark)`, in name order.
- `init` — load bundled and user themes, then apply the saved or default theme.
- `select` — apply a theme by name and remember the choice.
- `next_theme` — switch to the next theme in name order and remember the choice.
- `apply` — function
- `state_file` — function
- `user_theme_dir` — function
- `BUNDLED` — const

## Used by

- [[codewiki-app]] — from `follow_link`, `main`, `open_themes`, `palette_matches`

## See also

- [[_index]]
- [[codewiki-app]]
