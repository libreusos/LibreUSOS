//! Strings shared by more than one screen: navigation, sync status, the account menu and the
//! common buttons. Screen-specific text lives next to the screen.

use super::Text;

pub const APP_NAME: &str = "LibreUSOS";

// Daily navigation.
pub const NAV_TODAY: Text = Text::new("Dziś", "Today");
pub const NAV_WEEK: Text = Text::new("Plan", "Timetable");
pub const NAV_GRADES: Text = Text::new("Oceny", "Grades");
pub const NAV_TESTS: Text = Text::new("Sprawdziany", "Tests");

// Groups.
pub const GROUP_STUDIES: Text = Text::new("Studia", "Studies");
pub const GROUP_MATTERS: Text = Text::new("Sprawy", "Admin");
pub const GROUP_COMMS: Text = Text::new("Komunikacja", "Communication");

// Group items (all pending the core: shown disabled with `SOON`).
pub const NAV_DEADLINES: Text = Text::new("Terminy", "Deadlines");
pub const NAV_REGISTRATIONS: Text = Text::new("Rejestracje", "Registrations");
pub const NAV_SURVEYS: Text = Text::new("Ankiety", "Surveys");
pub const NAV_FIND_TIMETABLE: Text = Text::new("Wyszukaj plan", "Find a timetable");
pub const NAV_PAYMENTS: Text = Text::new("Płatności", "Payments");
pub const NAV_NEWS: Text = Text::new("Aktualności", "News");
pub const NAV_MESSAGES: Text = Text::new("Wiadomości", "Messages");
pub const NAV_SETTINGS: Text = Text::new("Ustawienia", "Settings");

/// The status word on a disabled item.
pub const SOON: Text = Text::new("wkrótce", "soon");
pub const SOON_TOOLTIP: Text = Text::new(
    "Ta funkcja będzie dostępna wkrótce",
    "This feature is coming soon",
);

pub const SEARCH: Text = Text::new("Szukaj", "Search");

// Sync status (sidebar footer, banners).
pub const SYNC_FRESH: Text = Text::new("Zsynchronizowano", "Synced");
pub const SYNC_SYNCING: Text = Text::new("Synchronizacja…", "Syncing…");
pub const SYNC_OFFLINE: Text = Text::new("Offline", "Offline");
pub const SYNC_EXPIRED: Text = Text::new("Sesja wygasła", "Session expired");
pub const SYNC_ERROR: Text = Text::new("Błąd synchronizacji", "Sync failed");
pub const SYNC_NEVER: Text = Text::new("Jeszcze nie zsynchronizowano", "Not synced yet");
pub const SYNC_NOW: Text = Text::new("Odśwież", "Refresh");
pub const SESSION_EXPIRED_BANNER: Text = Text::new(
    "Sesja USOS wygasła. Plan może być nieaktualny.",
    "Your USOS session expired. The timetable may be out of date.",
);
pub const SIGN_IN_AGAIN: Text = Text::new("Zaloguj ponownie", "Sign in again");
pub const SYNC_ERROR_BANNER: Text = Text::new(
    "Nie udało się pobrać danych. Sprawdź połączenie i spróbuj ponownie.",
    "Could not fetch your data. Check the connection and try again.",
);
pub const TRY_AGAIN: Text = Text::new("Spróbuj ponownie", "Try again");
pub const DETAILS: Text = Text::new("Szczegóły", "Details");

// Account menu.
pub const ACCOUNT: Text = Text::new("Konto", "Account");
pub const LOG_OUT: Text = Text::new("Wyloguj się", "Sign out");
pub const SWITCH_ACCOUNT: Text =
    Text::new("Zmień uczelnię lub konto", "Change university or account");
pub const LOG_OUT_TITLE: Text = Text::new("Wylogować się?", "Sign out?");
pub const LOG_OUT_BODY: Text = Text::new(
    "Dane zapisane na tym komputerze zostaną usunięte. Zalogujesz się ponownie, kiedy zechcesz.",
    "Data stored on this computer will be deleted. You can sign in again any time.",
);

// Common buttons.
pub const CANCEL: Text = Text::new("Anuluj", "Cancel");
pub const CONFIRM: Text = Text::new("Zatwierdź", "Confirm");

// Placeholder pages.

// Window controls tooltips.
pub const WINDOW_CLOSE: Text = Text::new("Zamknij okno", "Close window");
