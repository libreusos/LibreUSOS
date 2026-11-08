//! One view per route. A screen is an `Entity` whose `render` builds a [`crate::page::Page`];
//! the shell owns everything around it (sidebar, overlays, palette). Views read
//! `AppState::global(cx).read(cx).data` and observe `AppState` to repaint when it changes.

mod grades;
mod login;
pub(crate) mod settings;
mod tests;
mod today;
mod week;

pub use grades::GradesScreen;
pub use login::LoginScreen;
pub use settings::SettingsScreen;
pub use tests::TestsScreen;
pub use today::TodayScreen;
pub use week::WeekScreen;
