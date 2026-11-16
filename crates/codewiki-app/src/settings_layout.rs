//! The measures of the Settings page that are a matter of taste.
//!
//! Numbers copied from Material UI or TradingAgent stay in `mui/`, beside
//! the component they belong to, with their source. What is here has no
//! source to copy: how wide the category list is, how a row of a setting is
//! divided, how much room the entry button takes. They are in one place so
//! that they can be tuned by eye and changed together.

/// The list of categories at the left of the page.
pub const CATEGORY_LIST_WIDTH: f32 = 240.0;
/// Room around the list's rows, and above its first one.
pub const CATEGORY_LIST_PADDING: f32 = 8.0;
/// The line with the Back button and the word "Settings", above the list.
pub const HEADER_HEIGHT: f32 = 56.0;

/// Room around a category's page, and the widest the page gets.
pub const PANE_PADDING: f32 = 32.0;
pub const PANE_MEASURE: f32 = 760.0;

/// A row of one setting: its name and help at the left, its control at the
/// right, and the room between the two.
pub const LABEL_WIDTH: f32 = 240.0;
pub const CONTROL_WIDTH: f32 = 360.0;
pub const ROW_GAP: f32 = 24.0;
/// Between the rows of one card.
pub const ROW_SPACING: f32 = 20.0;

/// The "Settings" button at the foot of the sidebar and of Home: the room
/// around it.
pub const ENTRY_PADDING: f32 = 8.0;
