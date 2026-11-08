//! Screen-agnostic building blocks from SYSTEM.md section 10: buttons, badges, kind tags,
//! banners and the segmented control. They read colours from the active theme and never know
//! what a screen shows.

mod badge;
mod banner;
mod button;
mod empty_state;
mod grade_tile;
mod kbd;
mod menu;
mod progress;
mod segmented;
mod soon_card;
mod switch;
mod typography;
mod week_motif;

pub use badge::{Badge, BadgeKind, KindTag};
pub use banner::{Banner, BannerKind};
pub use button::{Btn, BtnStyle};
pub use empty_state::EmptyState;
pub use grade_tile::{GradeTile, TileKind};
pub use kbd::Kbd;
pub use menu::{MenuChip, MenuItem, MenuList};
pub use progress::ProgressBar;
pub use segmented::Segmented;
pub use soon_card::SoonCard;
pub use switch::Switch;
pub use typography::{mono_label, tnum};
pub use week_motif::{MOTIF_DAYS, WeekMotif};

use gpui::{App, ClickEvent, Window};

/// A click handler as the components store it.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
