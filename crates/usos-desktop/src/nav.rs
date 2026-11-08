//! Navigation: the [`Route`] enum, the history and the keyboard actions.
//!
//! A screen is one view per route (`screens/*.rs`); the shell builds the view for the current
//! route. Views change page with `Nav::go(route, cx)`.

use gpui::{App, AppContext as _, Context, Entity, Global, actions};

use crate::assets::icons;
use crate::i18n::{Text, strings};

actions!(
    usos,
    [
        GoToday,
        GoWeek,
        GoGrades,
        GoTests,
        OpenSettings,
        OpenPalette,
        Refresh,
        Back,
        Forward,
        OpenShortcuts,
        SwitchAccount,
        SignOut,
        Quit,
    ]
);

/// Every page of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    // Daily.
    Today,
    Week,
    Grades,
    Tests,
    // Studia.
    Deadlines,
    Registrations,
    Surveys,
    FindTimetable,
    // Sprawy.
    Payments,
    // Komunikacja.
    News,
    Messages,
    Settings,
}

/// A sidebar group (SYSTEM.md section 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Studies,
    Matters,
    Comms,
}

impl Group {
    pub const ALL: [Group; 3] = [Group::Studies, Group::Matters, Group::Comms];

    pub fn title(self) -> &'static Text {
        match self {
            Group::Studies => &strings::GROUP_STUDIES,
            Group::Matters => &strings::GROUP_MATTERS,
            Group::Comms => &strings::GROUP_COMMS,
        }
    }

    pub fn routes(self) -> &'static [Route] {
        match self {
            Group::Studies => &[
                Route::Deadlines,
                Route::Registrations,
                Route::Surveys,
                Route::FindTimetable,
            ],
            Group::Matters => &[Route::Payments],
            Group::Comms => &[Route::News, Route::Messages],
        }
    }
}

impl Route {
    /// The four daily items, Ctrl 1 to 4.
    pub const DAILY: [Route; 4] = [Route::Today, Route::Week, Route::Grades, Route::Tests];

    pub fn label(self) -> &'static Text {
        match self {
            Route::Today => &strings::NAV_TODAY,
            Route::Week => &strings::NAV_WEEK,
            Route::Grades => &strings::NAV_GRADES,
            Route::Tests => &strings::NAV_TESTS,
            Route::Deadlines => &strings::NAV_DEADLINES,
            Route::Registrations => &strings::NAV_REGISTRATIONS,
            Route::Surveys => &strings::NAV_SURVEYS,
            Route::FindTimetable => &strings::NAV_FIND_TIMETABLE,
            Route::Payments => &strings::NAV_PAYMENTS,
            Route::News => &strings::NAV_NEWS,
            Route::Messages => &strings::NAV_MESSAGES,
            Route::Settings => &strings::NAV_SETTINGS,
        }
    }

    /// The Material Symbols name (SYSTEM.md section 8).
    pub fn icon(self) -> &'static str {
        match self {
            Route::Today => icons::TODAY,
            Route::Week => icons::WEEK,
            Route::Grades => icons::GRADES,
            Route::Tests => icons::TESTS,
            Route::Deadlines => icons::DEADLINES,
            Route::Registrations => icons::REGISTRATIONS,
            Route::Surveys => icons::SURVEYS,
            Route::FindTimetable => icons::FIND_TIMETABLE,
            Route::Payments => icons::PAYMENTS,
            Route::News => icons::NEWS,
            Route::Messages => icons::MESSAGES,
            Route::Settings => icons::SETTINGS,
        }
    }

    /// The shortcut shown next to the item (without the modifier name), when it has one.
    pub fn shortcut(self) -> Option<&'static str> {
        match self {
            Route::Today => Some("Ctrl 1"),
            Route::Week => Some("Ctrl 2"),
            Route::Grades => Some("Ctrl 3"),
            Route::Tests => Some("Ctrl 4"),
            Route::Settings => Some("Ctrl ,"),
            _ => None,
        }
    }

    /// Whether the page has data behind it today. The others need API, cache or orchestration
    /// that `usos-core` does not offer yet, so the sidebar shows them disabled with "wkrótce".
    pub fn is_available(self) -> bool {
        matches!(
            self,
            Route::Today | Route::Week | Route::Grades | Route::Tests | Route::Settings
        )
    }
}

/// The current page and the way back.
pub struct Nav {
    route: Route,
    back: Vec<Route>,
    forward: Vec<Route>,
}

struct NavHandle(Entity<Nav>);

impl Global for NavHandle {}

impl Nav {
    /// Create the navigation state at Dziś. Call once at start-up.
    pub fn init(cx: &mut App) {
        let nav = cx.new(|_| Nav {
            route: Route::Today,
            back: Vec::new(),
            forward: Vec::new(),
        });
        cx.set_global(NavHandle(nav));
    }

    /// The navigation entity; observe it to repaint on a page change.
    pub fn global(cx: &App) -> Entity<Nav> {
        cx.global::<NavHandle>().0.clone()
    }

    pub fn route(&self) -> Route {
        self.route
    }

    /// Open `route`. Going to the current page does nothing. Pages that are not available yet
    /// cannot be opened.
    pub fn go(route: Route, cx: &mut App) {
        if !route.is_available() {
            return;
        }
        Self::global(cx).update(cx, |nav, cx| nav.open(route, cx));
    }

    /// Go back one page.
    pub fn go_back(cx: &mut App) {
        Self::global(cx).update(cx, |nav, cx| {
            if let Some(previous) = nav.back.pop() {
                nav.forward
                    .push(std::mem::replace(&mut nav.route, previous));
                cx.notify();
            }
        });
    }

    /// Undo a [`Nav::go_back`].
    pub fn go_forward(cx: &mut App) {
        Self::global(cx).update(cx, |nav, cx| {
            if let Some(next) = nav.forward.pop() {
                nav.back.push(std::mem::replace(&mut nav.route, next));
                cx.notify();
            }
        });
    }

    fn open(&mut self, route: Route, cx: &mut Context<Self>) {
        if route == self.route {
            return;
        }
        // Moving between the daily items is a switch, not a drill-down: no back stack growth.
        if !(Route::DAILY.contains(&route) && Route::DAILY.contains(&self.route)) {
            self.back.push(self.route);
            if self.back.len() > 32 {
                self.back.remove(0);
            }
        }
        self.forward.clear();
        self.route = route;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_items_have_shortcuts_one_to_four() {
        let shortcuts: Vec<_> = Route::DAILY.iter().map(|r| r.shortcut().unwrap()).collect();
        assert_eq!(shortcuts, ["Ctrl 1", "Ctrl 2", "Ctrl 3", "Ctrl 4"]);
    }

    #[test]
    fn grouped_items_wait_for_the_core() {
        for group in Group::ALL {
            for route in group.routes() {
                assert!(!route.is_available(), "{route:?}");
            }
        }
        for route in Route::DAILY {
            assert!(route.is_available());
        }
    }

    #[test]
    fn every_label_has_both_languages() {
        let all = Route::DAILY
            .into_iter()
            .chain(
                Group::ALL
                    .into_iter()
                    .flat_map(|g| g.routes().iter().copied()),
            )
            .chain([Route::Settings]);
        for route in all {
            let label = route.label();
            assert!(!label.pl.is_empty() && !label.en.is_empty(), "{route:?}");
        }
    }
}
