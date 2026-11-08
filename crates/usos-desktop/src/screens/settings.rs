//! Settings (d-06, d-16, d-19, d-21): a list of sections on the left, the chosen one on the
//! right. Appearance, accessibility and language are saved through `Prefs::update`;
//! notifications are shown disabled ("wkrótce") until the core has something to notify about.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Context, Global, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use material_colors::color::Rgb;

use crate::assets::{self, icons};
use crate::components::{Banner, BannerKind, Btn, BtnStyle, Segmented, Switch};
use crate::i18n::{I18nExt, Lang, Text, strings};
use crate::nav::{Nav, Refresh, Route, SignOut, SwitchAccount};
use crate::page::Page;
use crate::settings::{AccentChoice, LanguageChoice, Prefs, ThemeMode, Toggle};
use crate::shortcuts;
use crate::state::{AppState, SyncStatus};
use crate::theme::tokens::{BRAND_SEED, hsla, parse_hex};
use crate::theme::{self, FONT_MONO, Kind, ThemeExt};

/// The sections of the left list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Appearance,
    Notifications,
    Sync,
    Account,
    Shortcuts,
    About,
}

impl Section {
    const ALL: [Section; 6] = [
        Section::Appearance,
        Section::Notifications,
        Section::Sync,
        Section::Account,
        Section::Shortcuts,
        Section::About,
    ];

    fn title(self) -> &'static Text {
        const APPEARANCE: Text = Text::new("Wygląd", "Appearance");
        const NOTIFICATIONS: Text = Text::new("Powiadomienia", "Notifications");
        const SYNC: Text = Text::new("Synchronizacja", "Sync");
        const SHORTCUTS: Text = Text::new("Skróty klawiszowe", "Keyboard shortcuts");
        const ABOUT: Text = Text::new("O aplikacji", "About");
        match self {
            Section::Appearance => &APPEARANCE,
            Section::Notifications => &NOTIFICATIONS,
            Section::Sync => &SYNC,
            Section::Account => &strings::ACCOUNT,
            Section::Shortcuts => &SHORTCUTS,
            Section::About => &ABOUT,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::Appearance => icons::PALETTE,
            Section::Notifications => icons::NOTIFICATIONS,
            Section::Sync => icons::SYNC,
            Section::Account => icons::ACCOUNT,
            Section::Shortcuts => icons::KEYBOARD,
            Section::About => icons::INFO,
        }
    }
}

/// A section some other part of the app wants shown (the "Ta lista" shortcut opens Skróty).
struct Requested(Option<Section>);

impl Global for Requested {}

/// Open Settings on `section`.
pub fn open_section(section: Section, cx: &mut App) {
    cx.set_global(Requested(Some(section)));
    Nav::go(Route::Settings, cx);
    // Already on Settings: `Nav::go` changes nothing, so ask for a repaint.
    cx.refresh_windows();
}

const EYEBROW: Text = Text::new("USTAWIENIA", "SETTINGS");
const THEME_TITLE: Text = Text::new("Motyw", "Theme");
const THEME_CAPTION: Text = Text::new(
    "Domyślnie LibreUSOS robi to, co pulpit: jasny lub ciemny.",
    "By default LibreUSOS does what your desktop does: light or dark.",
);
const THEME_OPTIONS: [Text; 3] = [
    Text::new("Zgodnie z systemem", "Follow the system"),
    Text::new("Jasny", "Light"),
    Text::new("Ciemny", "Dark"),
];
const ACCENT_TITLE: Text = Text::new("Kolor akcentu", "Accent colour");
const ACCENT_CAPTION: Text = Text::new(
    "Z jednego koloru powstaje cały schemat, tak jak w Androidzie.",
    "One colour generates the whole scheme, as on Android.",
);
const ACCENT_SYSTEM: Text = Text::new("Z systemu", "From the system");
const ACCENT_SYSTEM_FOUND: Text = Text::new(
    "Pulpit podaje kolor akcentu:",
    "Your desktop provides the accent colour:",
);
const ACCENT_SYSTEM_MISSING: Text = Text::new(
    "Pulpit nie podaje koloru akcentu, więc używamy koloru LibreUSOS",
    "Your desktop provides no accent colour, so we use the LibreUSOS colour",
);
const ACCENT_BRAND: Text = Text::new("Kolor LibreUSOS", "LibreUSOS colour");
const ACCENT_BRAND_CAPTION: Text = Text::new("Granat z logo", "The navy of the logo");
const ACCENT_CUSTOM: Text = Text::new("Własny", "Custom");
const ACCENT_CUSTOM_CAPTION: Text = Text::new("Dowolny kolor", "Any colour");
const ACCENT_PICK: Text = Text::new("Wybierz…", "Choose…");
const ACCENT_HINT: Text = Text::new(
    "Na GNOME 47+ i KDE kolor akcentu przychodzi z ustawień pulpitu. Sway i większość menedżerów okien go nie mają.",
    "On GNOME 47+ and KDE the accent colour comes from the desktop settings. Sway and most window managers have none.",
);
const PREVIEW_TITLE: Text = Text::new("Podgląd", "Preview");
const PREVIEW_BUTTON: Text = Text::new("Przycisk", "Button");
const ACCESS_TITLE: Text = Text::new("Dostępność", "Accessibility");
const CONTRAST_TITLE: Text = Text::new("Podwyższony kontrast", "Increased contrast");
const MOTION_TITLE: Text = Text::new("Ograniczony ruch", "Reduced motion");
const LANGUAGE_TITLE: Text = Text::new("Język", "Language");
const CHOICE_SYSTEM: Text = Text::new("System", "System");
const CHOICE_ON: Text = Text::new("Włączony", "On");
const CHOICE_OFF: Text = Text::new("Wyłączony", "Off");
const FOLLOWING_ON: Text = Text::new(
    "Zgodnie z systemem · teraz włączony",
    "Follows the system · on now",
);
const FOLLOWING_OFF: Text = Text::new(
    "Zgodnie z systemem · teraz wyłączony",
    "Follows the system · off now",
);
const SET_ON: Text = Text::new("Włączony w LibreUSOS", "On in LibreUSOS");
const SET_OFF: Text = Text::new("Wyłączony w LibreUSOS", "Off in LibreUSOS");
const LANGUAGE_NAMES: [Text; 2] = [
    Text::new("Polski", "Polish"),
    Text::new("English", "English"),
];
const LANGUAGE_FOLLOWS: Text = Text::new("zgodnie z systemem", "follows the system");

const NOTIFICATIONS_SOON: Text = Text::new(
    "Powiadomienia pulpitu pojawią się wkrótce.",
    "Desktop notifications are coming soon.",
);
const KINDS_TITLE: Text = Text::new("Rodzaje", "Kinds");
const KINDS_CAPTION: Text = Text::new(
    "Te same co w telefonie. Pokazuje je usługa powiadomień pulpitu (portal XDG).",
    "The same as on the phone. Shown by the desktop's notification service (XDG portal).",
);
const WHEN_TITLE: Text = Text::new("Kiedy", "When");
const NOTIFY_ROWS: [(&str, Text, Text, bool); 4] = [
    (
        icons::GRADES,
        Text::new("Oceny", "Grades"),
        Text::new(
            "Nowa ocena końcowa lub poprawka",
            "A new final grade or retake",
        ),
        true,
    ),
    (
        icons::TESTS,
        Text::new("Sprawdziany", "Tests"),
        Text::new(
            "Nowe punkty; zbiorczo, raz na synchronizację",
            "New points; batched, once per sync",
        ),
        true,
    ),
    (
        "event_busy",
        Text::new("Zmiany w planie", "Timetable changes"),
        Text::new(
            "Odwołane lub przeniesione zajęcia w ciągu 7 dni",
            "Cancelled or moved classes within 7 days",
        ),
        true,
    ),
    (
        icons::SESSION,
        Text::new("Konto", "Account"),
        Text::new("Wygasła sesja USOS", "The USOS session expired"),
        true,
    ),
];
const QUIET_HOURS: (&str, Text, Text, bool) = (
    "bedtime",
    Text::new("Cisza nocna", "Quiet hours"),
    Text::new(
        "22:00–7:00 · powiadomienia czekają do rana",
        "22:00–7:00 · notifications wait until morning",
    ),
    true,
);
const SYNC_AFTER_CLOSE: (&str, Text, Text, bool) = (
    icons::SYNC,
    Text::new(
        "Synchronizuj po zamknięciu okna",
        "Sync after the window closes",
    ),
    Text::new(
        "Ikona w zasobniku, gdy pulpit go ma; inaczej tylko przy otwartym oknie",
        "A tray icon when the desktop has one; otherwise only while the window is open",
    ),
    false,
);

const SYNC_NOW_LABEL: Text = Text::new("Synchronizuj teraz", "Sync now");
const SYNC_NOW_TITLE: Text = Text::new("Dane z USOS", "Data from USOS");
const ACCOUNT_SIGNED_IN: Text = Text::new("Zalogowano", "Signed in");
const ABOUT_VERSION: Text = Text::new("Wersja", "Version");
const ABOUT_LICENSE: Text = Text::new("Licencja", "Licence");
const ABOUT_NOTE: Text = Text::new(
    "Wolny klient USOS. Dane pochodzą z USOS API twojej uczelni i zostają na tym komputerze.",
    "A free USOS client. The data comes from your university's USOS API and stays on this computer.",
);

/// Accent swatches of the custom picker (a hand-picked set instead of a free colour dial).
const SWATCHES: [&str; 10] = [
    "#3F5AA6", "#1E6FA8", "#00796B", "#557F3E", "#8A7A1E", "#B26A00", "#A5442F", "#B3306A",
    "#7A4EA6", "#5C5F66",
];

pub struct SettingsScreen {
    section: Section,
    picker_open: bool,
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            section: Section::Appearance,
            picker_open: false,
        }
    }
}

/// "#RRGGBB".
fn hex_of(color: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", color.red, color.green, color.blue)
}

/// The caption of a "follow the system" setting.
fn toggle_caption(choice: Toggle, effective: bool, lang: Lang) -> String {
    match (choice, effective) {
        (Toggle::System, true) => FOLLOWING_ON.render(lang).to_string(),
        (Toggle::System, false) => FOLLOWING_OFF.render(lang).to_string(),
        (Toggle::On, _) => SET_ON.render(lang).to_string(),
        (Toggle::Off, _) => SET_OFF.render(lang).to_string(),
    }
}

fn toggle_index(value: Toggle) -> usize {
    match value {
        Toggle::System => 0,
        Toggle::On => 1,
        Toggle::Off => 2,
    }
}

fn toggle_from(index: usize) -> Toggle {
    match index {
        1 => Toggle::On,
        2 => Toggle::Off,
        _ => Toggle::System,
    }
}

/// The caption of the language row: "Polski · zgodnie z systemem".
fn language_caption(choice: LanguageChoice, lang: Lang) -> String {
    let name = LANGUAGE_NAMES[usize::from(lang == Lang::En)].render(lang);
    match choice {
        LanguageChoice::System => format!("{name} · {}", LANGUAGE_FOLLOWS.render(lang)),
        _ => name.to_string(),
    }
}

/// The line under "Synchronizuj teraz".
fn sync_caption(status: &SyncStatus, lang: Lang) -> String {
    match status {
        SyncStatus::Fresh { at } => {
            format!(
                "{} {}",
                strings::SYNC_FRESH.render(lang),
                at.format("%H:%M")
            )
        }
        SyncStatus::Syncing => strings::SYNC_SYNCING.render(lang).to_string(),
        SyncStatus::Offline { .. } => strings::SYNC_OFFLINE.render(lang).to_string(),
        SyncStatus::Expired => strings::SYNC_EXPIRED.render(lang).to_string(),
        SyncStatus::Error { .. } => strings::SYNC_ERROR.render(lang).to_string(),
        SyncStatus::Idle => strings::SYNC_NEVER.render(lang).to_string(),
    }
}

fn labels(cx: &App, texts: &[&Text]) -> Vec<SharedString> {
    texts.iter().map(|text| cx.t(text)).collect()
}

/// A 16 px section heading with an optional grey line under it.
fn heading(title: SharedString, caption: Option<SharedString>, cx: &App) -> gpui::Div {
    let t = *cx.tokens();
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(title),
        )
        .children(caption.map(|caption| {
            div()
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(caption)
        }))
}

/// Rows in a rounded card with hairlines between them.
fn card(rows: Vec<AnyElement>, cx: &App) -> gpui::Div {
    let t = *cx.tokens();
    div()
        .flex()
        .flex_col()
        .rounded(px(12.0))
        .border_1()
        .border_color(t.outline_variant)
        .overflow_hidden()
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .when(index > 0, |d| d.border_t_1())
                .border_color(t.outline_variant)
                .child(row)
        }))
}

/// Icon, title, caption and a control at the right end.
fn list_row(
    icon: Option<&'static str>,
    title: SharedString,
    caption: Option<SharedString>,
    control: Option<AnyElement>,
    cx: &App,
) -> AnyElement {
    let t = *cx.tokens();
    div()
        .flex()
        .items_center()
        .gap(px(14.0))
        .min_h(px(56.0))
        .px(px(16.0))
        .py(px(8.0))
        .children(icon.map(|name| assets::icon(name, 24.0, false).text_color(t.on_surface_variant)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(px(14.0)).child(title))
                .children(caption.map(|caption| {
                    div()
                        .text_size(px(12.0))
                        .text_color(t.on_surface_variant)
                        .child(caption)
                })),
        )
        .children(control)
        .into_any_element()
}

/// One radio card of the accent picker.
fn accent_row(
    id: &'static str,
    selected: bool,
    title: SharedString,
    caption: SharedString,
    trailing: AnyElement,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let t = *cx.tokens();
    let radio = div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(20.0))
        .rounded_full()
        .border_2()
        .border_color(if selected { t.primary } else { t.outline })
        .children(selected.then(|| div().size(px(10.0)).rounded_full().bg(t.primary)));
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(14.0))
        .min_h(px(56.0))
        .px(px(16.0))
        .py(px(8.0))
        .cursor_pointer()
        .tab_stop(true)
        .focus_visible(|s| s.bg(t.surface_container_high))
        .hover(|s| s.bg(t.surface_container_high))
        .on_click(on_click)
        .child(radio)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(px(14.0)).child(title))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(t.on_surface_variant)
                        .child(caption),
                ),
        )
        .child(trailing)
        .into_any_element()
}

fn swatch(color: Rgb, size: f32) -> gpui::Div {
    div()
        .flex_none()
        .size(px(size))
        .rounded(px(5.0))
        .bg(hsla(color))
}

fn dispatch<A: gpui::Action>(action: A) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) {
    move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx)
}

impl SettingsScreen {
    fn appearance(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = *cx.tokens();
        let prefs = Prefs::global(cx).clone();
        let resolved = *cx.resolved_theme();
        let lang = cx.lang();

        let theme_picker = Segmented::new(
            "pref-theme",
            labels(cx, &THEME_OPTIONS.each_ref()),
            match prefs.theme_mode {
                ThemeMode::System => 0,
                ThemeMode::Light => 1,
                ThemeMode::Dark => 2,
            },
        )
        .on_select(|index, _, cx| {
            Prefs::update(cx, |prefs| {
                prefs.theme_mode = match index {
                    1 => ThemeMode::Light,
                    2 => ThemeMode::Dark,
                    _ => ThemeMode::System,
                }
            })
        });

        // Accent.
        let system_accent = theme::system_accent(cx);
        let custom = parse_hex(&prefs.custom_color).unwrap_or(BRAND_SEED);
        let system_caption: SharedString = match system_accent {
            Some(color) => format!("{} {}", cx.t(&ACCENT_SYSTEM_FOUND), hex_of(color)).into(),
            None => cx.t(&ACCENT_SYSTEM_MISSING),
        };
        let accent_rows = vec![
            accent_row(
                "accent-system",
                prefs.accent == AccentChoice::System,
                cx.t(&ACCENT_SYSTEM),
                system_caption,
                swatch(system_accent.unwrap_or(BRAND_SEED), 22.0).into_any_element(),
                |_, _, cx| Prefs::update(cx, |prefs| prefs.accent = AccentChoice::System),
                cx,
            ),
            accent_row(
                "accent-brand",
                prefs.accent == AccentChoice::Brand,
                cx.t(&ACCENT_BRAND),
                cx.t(&ACCENT_BRAND_CAPTION),
                swatch(BRAND_SEED, 22.0).into_any_element(),
                |_, _, cx| Prefs::update(cx, |prefs| prefs.accent = AccentChoice::Brand),
                cx,
            ),
            accent_row(
                "accent-custom",
                prefs.accent == AccentChoice::Custom,
                cx.t(&ACCENT_CUSTOM),
                cx.t(&ACCENT_CUSTOM_CAPTION),
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .when(prefs.accent == AccentChoice::Custom, |d| {
                        d.child(swatch(custom, 22.0))
                    })
                    .child(Btn::new("accent-pick", BtnStyle::Outlined).label(cx.t(&ACCENT_PICK)))
                    .into_any_element(),
                cx.listener(|this, _, _, cx| {
                    this.picker_open = !this.picker_open;
                    Prefs::update(cx, |prefs| prefs.accent = AccentChoice::Custom);
                    cx.notify();
                }),
                cx,
            ),
        ];
        let swatches = (self.picker_open && prefs.accent == AccentChoice::Custom).then(|| {
            div()
                .flex()
                .flex_wrap()
                .gap(px(10.0))
                .children(SWATCHES.iter().enumerate().map(|(index, hex)| {
                    let selected = prefs.custom_color.eq_ignore_ascii_case(hex);
                    let color = parse_hex(hex).unwrap_or(BRAND_SEED);
                    div()
                        .id(("swatch", index))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(32.0))
                        .rounded_full()
                        .bg(hsla(color))
                        .cursor_pointer()
                        .border_2()
                        .border_color(if selected { t.on_surface } else { hsla(color) })
                        .on_click(move |_, _, cx| {
                            Prefs::update(cx, |prefs| {
                                prefs.custom_color = (*hex).to_owned();
                                prefs.accent = AccentChoice::Custom;
                            })
                        })
                }))
        });

        // Preview: the three kinds on the generated scheme, and a button.
        let tile = |kind: Kind, code: &'static str, name: &'static str, room: &'static str| {
            let colors = t.kind(kind);
            div()
                .flex()
                .flex_col()
                .w(px(150.0))
                .p(px(10.0))
                .rounded(px(8.0))
                .bg(colors.fill)
                .text_color(colors.text)
                .when_some(colors.border, |d, border| d.border_1().border_color(border))
                .child(div().font_family(FONT_MONO).text_size(px(10.0)).child(code))
                .child(div().text_size(px(13.0)).child(name))
                .child(div().text_size(px(12.0)).opacity(0.75).child(room))
        };
        let preview = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .p(px(14.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(t.outline_variant)
            .child(tile(Kind::Lecture, "WYK", "Bazy danych", "Aula C"))
            .child(tile(Kind::Lab, "LAB", "Programowanie obiektowe", "s. 314"))
            .child(tile(Kind::Other, "ĆW", "Matematyka dyskretna", "s. 216"))
            .child(div().flex_1())
            .child(Btn::new("preview-button", BtnStyle::Filled).label(cx.t(&PREVIEW_BUTTON)));

        // Accessibility.
        let three = |id: &'static str, value: Toggle, set: fn(&mut Prefs, Toggle)| {
            Segmented::new(
                id,
                labels(cx, &[&CHOICE_SYSTEM, &CHOICE_ON, &CHOICE_OFF]),
                toggle_index(value),
            )
            .on_select(move |index, _, cx| {
                Prefs::update(cx, |prefs| set(prefs, toggle_from(index)))
            })
            .into_any_element()
        };
        let language_picker = Segmented::new(
            "pref-language",
            labels(
                cx,
                &[&CHOICE_SYSTEM, &LANGUAGE_NAMES[0], &LANGUAGE_NAMES[1]],
            ),
            match prefs.language {
                LanguageChoice::System => 0,
                LanguageChoice::Pl => 1,
                LanguageChoice::En => 2,
            },
        )
        .on_select(|index, _, cx| {
            Prefs::update(cx, |prefs| {
                prefs.language = match index {
                    1 => LanguageChoice::Pl,
                    2 => LanguageChoice::En,
                    _ => LanguageChoice::System,
                }
            })
        });
        let access = vec![
            list_row(
                Some(icons::CONTRAST),
                cx.t(&CONTRAST_TITLE),
                Some(toggle_caption(prefs.high_contrast, resolved.contrast > 0.0, lang).into()),
                Some(three("pref-contrast", prefs.high_contrast, |p, v| {
                    p.high_contrast = v
                })),
                cx,
            ),
            list_row(
                Some("motion_photos_off"),
                cx.t(&MOTION_TITLE),
                Some(toggle_caption(prefs.reduced_motion, resolved.reduced_motion, lang).into()),
                Some(three("pref-motion", prefs.reduced_motion, |p, v| {
                    p.reduced_motion = v
                })),
                cx,
            ),
            list_row(
                Some(icons::TRANSLATE),
                cx.t(&LANGUAGE_TITLE),
                Some(language_caption(prefs.language, lang).into()),
                Some(language_picker.into_any_element()),
                cx,
            ),
        ];

        vec![
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(heading(cx.t(&THEME_TITLE), Some(cx.t(&THEME_CAPTION)), cx))
                .child(div().flex().child(theme_picker))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(heading(
                    cx.t(&ACCENT_TITLE),
                    Some(cx.t(&ACCENT_CAPTION)),
                    cx,
                ))
                .child(card(accent_rows, cx))
                .children(swatches)
                .children(system_accent.is_none().then(|| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .text_size(px(12.0))
                        .text_color(t.on_surface_variant)
                        .child(assets::icon(icons::INFO, 18.0, false))
                        .child(cx.t(&ACCENT_HINT))
                }))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(t.on_surface_variant)
                        .child(cx.t(&PREVIEW_TITLE)),
                )
                .child(preview)
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(heading(cx.t(&ACCESS_TITLE), None, cx))
                .child(card(access, cx))
                .into_any_element(),
        ]
    }

    /// Notifications are shown as designed but cannot be changed yet.
    fn notifications(&self, cx: &App) -> Vec<AnyElement> {
        let row = |(icon, title, caption, on): &(&'static str, Text, Text, bool), index: usize| {
            list_row(
                Some(icon),
                cx.t(title),
                Some(cx.t(caption)),
                Some(
                    Switch::new(("notify", index), *on)
                        .disabled(true)
                        .into_any_element(),
                ),
                cx,
            )
        };
        vec![
            Banner::new(
                BannerKind::Info,
                icons::NOTIFICATIONS,
                cx.t(&NOTIFICATIONS_SOON),
            )
            .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(heading(cx.t(&KINDS_TITLE), Some(cx.t(&KINDS_CAPTION)), cx))
                .child(card(
                    NOTIFY_ROWS
                        .iter()
                        .enumerate()
                        .map(|(index, entry)| row(entry, index))
                        .collect(),
                    cx,
                ))
                .into_any_element(),
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(heading(cx.t(&WHEN_TITLE), None, cx))
                .child(card(
                    vec![row(&QUIET_HOURS, 4), row(&SYNC_AFTER_CLOSE, 5)],
                    cx,
                ))
                .into_any_element(),
        ]
    }

    fn sync(&self, cx: &App) -> Vec<AnyElement> {
        let status = AppState::global(cx).read(cx).sync.clone();
        let syncing = status == SyncStatus::Syncing;
        vec![
            card(
                vec![list_row(
                    Some(icons::SYNC),
                    cx.t(&SYNC_NOW_TITLE),
                    Some(sync_caption(&status, cx.lang()).into()),
                    Some(
                        Btn::new("sync-now", BtnStyle::Outlined)
                            .label(cx.t(&SYNC_NOW_LABEL))
                            .disabled(syncing)
                            .on_click(dispatch(Refresh))
                            .into_any_element(),
                    ),
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn account(&self, cx: &App) -> Vec<AnyElement> {
        let state = AppState::global(cx).read(cx);
        let name = state
            .user_name()
            .unwrap_or_else(|| cx.t(&strings::ACCOUNT).to_string());
        let university = state.university();
        vec![
            card(
                vec![
                    list_row(
                        Some(icons::ACCOUNT),
                        name.into(),
                        Some(format!("{} · {university}", cx.t(&ACCOUNT_SIGNED_IN)).into()),
                        None,
                        cx,
                    ),
                    list_row(
                        Some(icons::SWITCH),
                        cx.t(&strings::SWITCH_ACCOUNT),
                        None,
                        Some(
                            Btn::new("settings-switch", BtnStyle::Outlined)
                                .label(cx.t(&strings::SWITCH_ACCOUNT))
                                .on_click(dispatch(SwitchAccount))
                                .into_any_element(),
                        ),
                        cx,
                    ),
                    list_row(
                        Some(icons::LOGOUT),
                        cx.t(&strings::LOG_OUT),
                        None,
                        Some(
                            Btn::new("settings-sign-out", BtnStyle::Danger)
                                .label(cx.t(&strings::LOG_OUT))
                                .on_click(dispatch(SignOut))
                                .into_any_element(),
                        ),
                        cx,
                    ),
                ],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn about(&self, cx: &App) -> Vec<AnyElement> {
        let t = *cx.tokens();
        let line = |label: &Text, value: &str| {
            list_row(
                None,
                cx.t(label),
                None,
                Some(
                    div()
                        .text_size(px(13.0))
                        .font_family(FONT_MONO)
                        .text_color(t.on_surface_variant)
                        .child(value.to_owned())
                        .into_any_element(),
                ),
                cx,
            )
        };
        vec![
            card(
                vec![
                    line(&ABOUT_VERSION, env!("CARGO_PKG_VERSION")),
                    line(&ABOUT_LICENSE, env!("CARGO_PKG_LICENSE")),
                ],
                cx,
            )
            .into_any_element(),
            div()
                .text_size(px(12.5))
                .text_color(t.on_surface_variant)
                .child(cx.t(&ABOUT_NOTE))
                .into_any_element(),
        ]
    }

    fn section_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.tokens();
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(2.0))
            .w(px(220.0))
            .p(px(12.0))
            .border_r_1()
            .border_color(t.outline_variant)
            .children(
                Section::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, section)| {
                        let selected = section == self.section;
                        div()
                            .id(("settings-section", index))
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .h(px(36.0))
                            .px(px(12.0))
                            .rounded(px(10.0))
                            .text_size(px(13.5))
                            .cursor_pointer()
                            .tab_stop(true)
                            .focus_visible(|s| s.border_2().border_color(t.primary))
                            .when(selected, |d| {
                                d.bg(t.secondary_container)
                                    .text_color(t.on_secondary_container)
                            })
                            .when(!selected, |d| d.hover(|s| s.bg(t.surface_container_high)))
                            .child(assets::icon(section.icon(), 20.0, selected))
                            .child(cx.t(section.title()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.section = section;
                                cx.notify();
                            }))
                    }),
            )
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.tokens();
        if let Some(Requested(Some(section))) = cx.try_global::<Requested>() {
            self.section = *section;
            cx.set_global(Requested(None));
        }
        let content: Vec<AnyElement> = match self.section {
            Section::Appearance => self.appearance(cx),
            Section::Notifications => self.notifications(cx),
            Section::Sync => self.sync(cx),
            Section::Account => self.account(cx),
            Section::Shortcuts => vec![shortcuts::view(cx).into_any_element()],
            Section::About => self.about(cx),
        };
        Page::new(cx.t(self.section.title()))
            .eyebrow(cx.t(&EYEBROW))
            .without_banners()
            .child(
                div()
                    .flex()
                    .size_full()
                    .min_h_0()
                    .child(self.section_list(cx))
                    .child(
                        div()
                            .id("settings-content")
                            .flex_1()
                            .min_w_0()
                            .overflow_y_scroll()
                            .text_color(t.on_surface)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(28.0))
                                    .max_w(px(760.0))
                                    .p(px(24.0))
                                    .children(content),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    #[test]
    fn hex_is_upper_case_and_padded() {
        assert_eq!(hex_of(Rgb::new(0x55, 0x7F, 0x3E)), "#557F3E");
        assert_eq!(hex_of(Rgb::new(0, 1, 2)), "#000102");
    }

    #[test]
    fn every_swatch_parses_and_survives_a_round_trip() {
        for hex in SWATCHES {
            let color = parse_hex(hex).unwrap_or_else(|| panic!("{hex}"));
            assert_eq!(hex_of(color), hex);
        }
    }

    #[test]
    fn the_brand_colour_is_the_first_swatch() {
        assert_eq!(hex_of(BRAND_SEED), SWATCHES[0]);
    }

    #[test]
    fn toggles_round_trip_through_the_segmented_index() {
        for value in [Toggle::System, Toggle::On, Toggle::Off] {
            assert_eq!(toggle_from(toggle_index(value)), value);
        }
        assert_eq!(toggle_from(9), Toggle::System);
    }

    #[test]
    fn toggle_caption_says_what_the_system_does_only_when_following_it() {
        assert_eq!(
            toggle_caption(Toggle::System, false, Lang::Pl),
            "Zgodnie z\u{a0}systemem · teraz wyłączony"
        );
        assert_eq!(
            toggle_caption(Toggle::System, true, Lang::En),
            "Follows the system · on now"
        );
        assert_eq!(
            toggle_caption(Toggle::On, false, Lang::Pl),
            "Włączony w\u{a0}LibreUSOS"
        );
        assert_eq!(
            toggle_caption(Toggle::Off, true, Lang::En),
            "Off in LibreUSOS"
        );
    }

    #[test]
    fn language_caption_names_the_language_and_the_follow() {
        assert_eq!(
            language_caption(LanguageChoice::System, Lang::Pl),
            "Polski · zgodnie z\u{a0}systemem"
        );
        assert_eq!(language_caption(LanguageChoice::En, Lang::En), "English");
        assert_eq!(language_caption(LanguageChoice::Pl, Lang::Pl), "Polski");
    }

    #[test]
    fn sync_caption_shows_the_time_only_when_fresh() {
        let at = Local.with_ymd_and_hms(2026, 11, 8, 10, 52, 0).unwrap();
        assert_eq!(
            sync_caption(&SyncStatus::Fresh { at }, Lang::Pl),
            "Zsynchronizowano 10:52"
        );
        assert_eq!(
            sync_caption(&SyncStatus::Idle, Lang::Pl),
            "Jeszcze nie zsynchronizowano"
        );
        assert_eq!(
            sync_caption(&SyncStatus::Expired, Lang::En),
            "Session expired"
        );
    }

    #[test]
    fn every_section_has_a_title_and_an_icon() {
        for section in Section::ALL {
            assert!(!section.title().pl.is_empty());
            assert!(!section.icon().is_empty());
        }
    }
}
