//! Which tabs stay when some are closed: the logic of the tab menu, kept
//! apart from the view so that it can be tested.

/// Which tabs to close, named by the tab the user acted on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Close {
    /// That tab.
    One(usize),
    /// Every tab but that one.
    Others(usize),
    /// The tabs after that one.
    Right(usize),
    /// The tabs before that one.
    Left(usize),
    /// Every tab. The app keeps one tab, which the caller makes anew.
    All,
}

impl Close {
    /// The row of the tab menu that does it.
    pub fn label(self) -> &'static str {
        match self {
            Close::One(_) => "Close",
            Close::Others(_) => "Close others",
            Close::Right(_) => "Close to the right",
            Close::Left(_) => "Close to the left",
            Close::All => "Close all",
        }
    }
}

/// The ways to close tabs that the menu of the tab at `index`, of `count`,
/// offers: those that close something.
pub fn offered(count: usize, index: usize) -> Vec<Close> {
    [
        Close::One(index),
        Close::Others(index),
        Close::Right(index),
        Close::Left(index),
        Close::All,
    ]
    .into_iter()
    .filter(|&close| plan(count, index, close).is_some())
    .collect()
}

/// What is left after closing.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The tabs that stay, by their index before, in their order. Empty
    /// after [`Close::All`].
    pub keep: Vec<usize>,
    /// The active tab after, as an index into `keep`.
    pub active: usize,
    /// The active tab was closed, so its page is left and another tab's
    /// page shows.
    pub active_closed: bool,
}

/// What closing does to `count` tabs of which `active` is shown. `None`
/// when it would close nothing, or the last tab by itself.
pub fn plan(count: usize, active: usize, close: Close) -> Option<Outcome> {
    if count == 0 || active >= count {
        return None;
    }
    let target = match close {
        Close::One(index) | Close::Others(index) | Close::Right(index) | Close::Left(index) => {
            Some(index)
        }
        Close::All => None,
    };
    if target.is_some_and(|index| index >= count) {
        return None;
    }
    let keep: Vec<usize> = (0..count)
        .filter(|&index| match close {
            Close::One(target) => index != target,
            Close::Others(target) => index == target,
            Close::Right(target) => index <= target,
            Close::Left(target) => index >= target,
            Close::All => false,
        })
        .collect();
    if keep.len() == count || (keep.is_empty() && target.is_some()) {
        return None;
    }
    let (active_after, active_closed) = match keep.iter().position(|&index| index == active) {
        Some(position) => (position, false),
        // The tab that took the closed one's place, or the last.
        None if matches!(close, Close::One(_)) => (active.min(keep.len() - 1), true),
        // The tab the user acted on stays, and shows.
        None => (
            target
                .and_then(|target| keep.iter().position(|&index| index == target))
                .unwrap_or(0),
            true,
        ),
    };
    Some(Outcome {
        keep,
        active: active_after,
        active_closed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(keep: &[usize], active: usize, active_closed: bool) -> Option<Outcome> {
        Some(Outcome {
            keep: keep.to_vec(),
            active,
            active_closed,
        })
    }

    #[test]
    fn closing_a_tab_before_the_active_one_keeps_it_shown() {
        assert_eq!(plan(3, 2, Close::One(0)), outcome(&[1, 2], 1, false));
    }

    #[test]
    fn closing_the_active_tab_shows_the_one_after_it_or_the_last() {
        assert_eq!(plan(3, 1, Close::One(1)), outcome(&[0, 2], 1, true));
        assert_eq!(plan(3, 2, Close::One(2)), outcome(&[0, 1], 1, true));
    }

    #[test]
    fn the_last_tab_does_not_close_by_itself() {
        assert_eq!(plan(1, 0, Close::One(0)), None);
        assert_eq!(plan(1, 0, Close::Others(0)), None);
    }

    #[test]
    fn closing_the_others_keeps_the_tab_acted_on_and_shows_it() {
        assert_eq!(plan(4, 0, Close::Others(2)), outcome(&[2], 0, true));
        assert_eq!(plan(4, 2, Close::Others(2)), outcome(&[2], 0, false));
    }

    #[test]
    fn closing_to_the_right_keeps_the_tabs_up_to_the_one_acted_on() {
        assert_eq!(plan(4, 0, Close::Right(1)), outcome(&[0, 1], 0, false));
        assert_eq!(plan(4, 3, Close::Right(1)), outcome(&[0, 1], 1, true));
        assert_eq!(plan(4, 0, Close::Right(3)), None);
    }

    #[test]
    fn closing_to_the_left_keeps_the_tabs_from_the_one_acted_on() {
        assert_eq!(plan(4, 3, Close::Left(2)), outcome(&[2, 3], 1, false));
        assert_eq!(plan(4, 0, Close::Left(2)), outcome(&[2, 3], 0, true));
        assert_eq!(plan(4, 0, Close::Left(0)), None);
    }

    #[test]
    fn closing_all_keeps_nothing_even_of_one_tab() {
        assert_eq!(plan(3, 1, Close::All), outcome(&[], 0, true));
        assert_eq!(plan(1, 0, Close::All), outcome(&[], 0, true));
    }

    #[test]
    fn the_menu_offers_only_what_closes_something() {
        use Close::*;
        assert_eq!(offered(1, 0), [All]);
        assert_eq!(offered(3, 0), [One(0), Others(0), Right(0), All]);
        assert_eq!(offered(3, 1), [One(1), Others(1), Right(1), Left(1), All]);
        assert_eq!(offered(3, 2), [One(2), Others(2), Left(2), All]);
    }

    #[test]
    fn a_tab_that_is_not_there_closes_nothing() {
        assert_eq!(plan(2, 0, Close::One(5)), None);
        assert_eq!(plan(2, 0, Close::Others(2)), None);
        assert_eq!(plan(0, 0, Close::All), None);
    }
}
