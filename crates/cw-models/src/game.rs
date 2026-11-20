//! Whether a game is running.
//!
//! A model's worker holds memory, and embedding in the background takes the
//! processor or the graphics card. While the user plays, both give way: the
//! worker does not linger, and a background job waits.
//!
//! A game is two things at once:
//!
//! 1. Windows says a full-screen app is in front. This is what Windows
//!    itself goes by to hold notifications back.
//! 2. The programs other than this one keep the graphics card's 3D engines
//!    busy.
//!
//! One without the other is not a game: a full-screen video leaves the card
//! almost idle, and a busy card with no full-screen app is some other work.
//! The load must also last, so that a scene change in a video counts for
//! nothing.
//!
//! [`Detector`] is the rule, over readings it is given. The readings come
//! from the two Win32 calls in the `win32` module below.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Percent of a 3D engine that the other programs must keep busy.
const LOAD: f64 = 50.0;
/// How long the load must last.
const SUSTAIN: Duration = Duration::from_secs(15);
/// After this long without a look, what was seen before counts no more.
const FORGET: Duration = Duration::from_secs(30);
/// A look is this old at the most; who asks sooner gets the last answer.
/// One look takes a second, to measure a rate.
const FRESH: Duration = Duration::from_secs(4);

/// What the machine says at one moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// Windows reports a full-screen app in front.
    pub full_screen: bool,
    /// Percent of the 3D engines in use by other programs; `None` when it
    /// was not read, or could not be.
    pub load_3d: Option<f64>,
}

/// The rule: both readings, for long enough.
#[derive(Debug, Default)]
pub struct Detector {
    /// Since when both hold without a break.
    since: Option<Instant>,
    last_look: Option<Instant>,
}

impl Detector {
    /// Take one reading at `now` and say whether a game is running.
    pub fn observe(&mut self, now: Instant, reading: Reading) -> bool {
        if self
            .last_look
            .is_some_and(|last| now.duration_since(last) > FORGET)
        {
            self.since = None;
        }
        self.last_look = Some(now);
        let both = reading.full_screen && reading.load_3d.is_some_and(|load| load >= LOAD);
        if !both {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now);
        now.duration_since(since) >= SUSTAIN
    }
}

struct Watch {
    detector: Detector,
    /// The last answer and when it was worked out.
    answer: Option<(Instant, bool)>,
}

/// Whether a game is running on this machine. Cheap to ask often: the
/// machine is looked at every few seconds at the most, and its graphics load
/// only while a full-screen app is in front. Always false where the readings
/// do not exist.
pub fn busy() -> bool {
    static WATCH: Mutex<Watch> = Mutex::new(Watch {
        detector: Detector {
            since: None,
            last_look: None,
        },
        answer: None,
    });
    let mut watch = WATCH
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((at, answer)) = watch.answer
        && at.elapsed() < FRESH
    {
        return answer;
    }
    let full_screen = win32::full_screen();
    let reading = Reading {
        full_screen,
        // A second of measuring, spent only when it can matter.
        load_3d: if full_screen {
            win32::load_3d_of_others()
        } else {
            None
        },
    };
    let now = Instant::now();
    let answer = watch.detector.observe(now, reading);
    watch.answer = Some((now, answer));
    answer
}

#[cfg(not(windows))]
mod win32 {
    pub fn full_screen() -> bool {
        false
    }

    pub fn load_3d_of_others() -> Option<f64> {
        None
    }
}

/// The two readings. With `cw_core::secret` and the app's `win32.rs`, the
/// only modules of the workspace with `unsafe` code.
#[cfg(windows)]
#[allow(unsafe_code)]
mod win32 {
    use std::time::Duration;

    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW,
    };
    use windows_sys::Win32::UI::Shell::{
        QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
    };

    /// Whether Windows reports a Direct3D full-screen app, or a full-screen
    /// app, in front. A borderless full-screen game is the second.
    pub fn full_screen() -> bool {
        let mut state = 0;
        // SAFETY: the function writes one i32 through a pointer that is
        // valid for the call.
        let result = unsafe { SHQueryUserNotificationState(&mut state) };
        result >= 0 && matches!(state, QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_BUSY)
    }

    /// Closes the query when it goes out of scope, on every path.
    struct Query(PDH_HQUERY);

    impl Drop for Query {
        fn drop(&mut self) {
            // SAFETY: the handle came from PdhOpenQueryW and is closed once.
            unsafe { PdhCloseQuery(self.0) };
        }
    }

    /// Percent of the graphics card's 3D engines that programs other than
    /// this one used over the last second, added up. Windows keeps one
    /// counter for each program and engine, named
    /// `pid_<id>_luid_..._engtype_3D`. A model on the graphics card works
    /// through other engines, so the worker's own embedding is not in it.
    pub fn load_3d_of_others() -> Option<f64> {
        let path: Vec<u16> = r"\GPU Engine(*engtype_3D)\Utilization Percentage"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let own = std::process::id();
        // SAFETY: every pointer is to a local that outlives the call it is
        // passed to. The buffer is as large as PDH asked for, and aligned
        // for the items PDH writes at its start; the names PDH points to
        // lie in the same buffer and end with a zero.
        unsafe {
            let mut handle: PDH_HQUERY = std::ptr::null_mut();
            if PdhOpenQueryW(std::ptr::null(), 0, &mut handle) != 0 {
                return None;
            }
            let query = Query(handle);
            let mut counter: PDH_HCOUNTER = std::ptr::null_mut();
            if PdhAddEnglishCounterW(query.0, path.as_ptr(), 0, &mut counter) != 0 {
                return None;
            }
            // A rate needs two samples.
            PdhCollectQueryData(query.0);
            std::thread::sleep(Duration::from_secs(1));
            if PdhCollectQueryData(query.0) != 0 {
                return None;
            }
            let mut size = 0u32;
            let mut count = 0u32;
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            );
            if status != PDH_MORE_DATA {
                return None;
            }
            let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
            let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, items)
                != 0
            {
                return None;
            }
            let mut total = 0.0;
            for index in 0..count as usize {
                let item = &*items.add(index);
                let mut length = 0;
                while *item.szName.add(length) != 0 {
                    length += 1;
                }
                let name =
                    String::from_utf16_lossy(std::slice::from_raw_parts(item.szName, length));
                let pid = name
                    .strip_prefix("pid_")
                    .and_then(|rest| rest.split('_').next())
                    .and_then(|pid| pid.parse::<u32>().ok());
                if pid != Some(own) {
                    total += item.FmtValue.Anonymous.doubleValue;
                }
            }
            Some(total)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(full_screen: bool, load: f64) -> Reading {
        Reading {
            full_screen,
            load_3d: Some(load),
        }
    }

    /// Readings five seconds apart, as the watch takes them.
    fn observe_all(detector: &mut Detector, start: Instant, readings: &[Reading]) -> Vec<bool> {
        readings
            .iter()
            .enumerate()
            .map(|(index, reading)| {
                detector.observe(start + Duration::from_secs(5 * index as u64), *reading)
            })
            .collect()
    }

    #[test]
    fn a_game_is_a_full_screen_app_and_a_busy_card_for_long_enough() {
        let game = reading(true, 85.0);
        let answers = observe_all(&mut Detector::default(), Instant::now(), &[game; 5]);
        // 0, 5 and 10 seconds are not long enough; 15 is.
        assert_eq!(answers, [false, false, false, true, true]);
    }

    #[test]
    fn one_without_the_other_is_no_game() {
        // A full-screen video: the card is almost idle.
        let video = reading(true, 6.0);
        // A busy card behind a desktop: some other work.
        let render = reading(false, 95.0);
        // Full screen, and the load could not be read.
        let unread = Reading {
            full_screen: true,
            load_3d: None,
        };
        for other in [video, render, unread, reading(false, 0.0)] {
            let answers = observe_all(&mut Detector::default(), Instant::now(), &[other; 6]);
            assert!(answers.iter().all(|answer| !answer), "{other:?}");
        }
        // Just at the line counts.
        let answers = observe_all(
            &mut Detector::default(),
            Instant::now(),
            &[reading(true, 50.0); 4],
        );
        assert_eq!(answers.last(), Some(&true));
    }

    #[test]
    fn a_break_starts_the_count_again() {
        let game = reading(true, 85.0);
        let pause = reading(true, 3.0);
        let answers = observe_all(
            &mut Detector::default(),
            Instant::now(),
            &[game, game, game, pause, game, game, game, game],
        );
        assert_eq!(
            answers,
            [false, false, false, false, false, false, false, true]
        );
    }

    #[test]
    fn the_end_of_the_game_is_seen_at_once() {
        let mut detector = Detector::default();
        let start = Instant::now();
        let game = reading(true, 85.0);
        assert_eq!(
            observe_all(&mut detector, start, &[game; 4]).last(),
            Some(&true)
        );
        let after = start + Duration::from_secs(20);
        assert!(!detector.observe(after, reading(false, 85.0)));
    }

    #[test]
    fn what_was_seen_long_ago_does_not_count() {
        let mut detector = Detector::default();
        let start = Instant::now();
        let game = reading(true, 85.0);
        assert!(!detector.observe(start, game));
        // Nobody looked for an hour. The game may have ended and another
        // begun: the count starts again.
        let later = start + Duration::from_secs(3600);
        assert!(!detector.observe(later, game));
        assert!(!detector.observe(later + Duration::from_secs(10), game));
        assert!(detector.observe(later + Duration::from_secs(15), game));
    }

    #[test]
    fn the_machine_can_be_asked() {
        // Whatever the answer on the machine that runs the tests, asking
        // must not fail, and asking again at once gives the same answer.
        let first = busy();
        assert_eq!(busy(), first);
    }
}
