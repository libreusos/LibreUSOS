//! The command palette (Ctrl K, d-08): a search field over the screens and a few commands. It
//! only lists commands; the shell runs them (it owns the confirmation dialogs). Screens that
//! are not built yet are listed last, disabled, with "wkrótce".

mod matching;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Window,
    div, px,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::assets::{self, icons};
use crate::components::Kbd;
use crate::i18n::{I18nExt, Lang, Text, strings};
use crate::nav::{Group as NavGroup, Route};
use crate::settings::{Prefs, ThemeMode};
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt};

const PLACEHOLDER: Text = Text::new(
    "Wpisz nazwę ekranu lub polecenie",
    "Type a screen name or a command",
);
const NO_RESULTS: Text = Text::new("Brak wyników", "No results");
const SYNC_NOW: Text = Text::new("Synchronizuj teraz", "Sync now");
const SHORTCUTS: Text = Text::new("Skróty klawiszowe", "Keyboard shortcuts");
const TOGGLE_TO_DARK: Text = Text::new("Przełącz motyw: ciemny", "Switch theme: dark");
const TOGGLE_TO_LIGHT: Text = Text::new("Przełącz motyw: jasny", "Switch theme: light");
const NOW_SYSTEM: Text = Text::new("Teraz: zgodnie z systemem", "Now: follows the system");
const NOW_LIGHT: Text = Text::new("Teraz: jasny", "Now: light");
const NOW_DARK: Text = Text::new("Teraz: ciemny", "Now: dark");
const HINT_MOVE: Text = Text::new("wybór", "select");
const HINT_OPEN: Text = Text::new("otwórz", "open");
const HINT_CLOSE: Text = Text::new("zamknij", "close");

/// What the palette can ask the shell to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Go(Route),
    Refresh,
    ToggleTheme,
    Shortcuts,
    SwitchAccount,
    SignOut,
}

/// Light when the theme is dark now, else dark (what "Przełącz motyw" does).
pub fn toggle_theme(cx: &mut App) {
    let dark = cx.resolved_theme().dark;
    Prefs::update(cx, |prefs| {
        prefs.theme_mode = if dark {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        }
    });
}

/// The headed sections of the list, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Section {
    Screens,
    Commands,
    /// Not available yet: shown, disabled.
    Soon,
}

impl Section {
    fn title(self) -> &'static Text {
        const SCREENS: Text = Text::new("EKRANY", "SCREENS");
        const COMMANDS: Text = Text::new("POLECENIA", "COMMANDS");
        const SOON: Text = Text::new("WKRÓTCE", "COMING SOON");
        match self {
            Section::Screens => &SCREENS,
            Section::Commands => &COMMANDS,
            Section::Soon => &SOON,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Item {
    command: Command,
    section: Section,
    label: String,
    subtitle: Option<String>,
    icon: &'static str,
    shortcut: Option<&'static str>,
}

impl Item {
    fn enabled(&self) -> bool {
        self.section != Section::Soon
    }
}

/// Every entry of the palette for the current language and theme.
fn items(lang: Lang, dark: bool, mode: ThemeMode) -> Vec<Item> {
    let go = |route: Route, section: Section| Item {
        command: Command::Go(route),
        section,
        label: route.label().render(lang).to_string(),
        subtitle: None,
        icon: route.icon(),
        shortcut: route.shortcut(),
    };
    let command = |command: Command,
                   label: &Text,
                   subtitle: Option<&Text>,
                   icon: &'static str,
                   shortcut: Option<&'static str>| Item {
        command,
        section: Section::Commands,
        label: label.render(lang).to_string(),
        subtitle: subtitle.map(|text| text.render(lang).to_string()),
        icon,
        shortcut,
    };
    let mut items: Vec<Item> = Route::DAILY
        .into_iter()
        .chain([Route::Settings])
        .map(|route| go(route, Section::Screens))
        .collect();
    items.extend([
        command(
            Command::Refresh,
            &SYNC_NOW,
            None,
            icons::SYNC,
            Some("Ctrl R"),
        ),
        command(
            Command::ToggleTheme,
            if dark {
                &TOGGLE_TO_LIGHT
            } else {
                &TOGGLE_TO_DARK
            },
            Some(match mode {
                ThemeMode::System => &NOW_SYSTEM,
                ThemeMode::Light => &NOW_LIGHT,
                ThemeMode::Dark => &NOW_DARK,
            }),
            if dark {
                icons::LIGHT_MODE
            } else {
                icons::DARK_MODE
            },
            None,
        ),
        command(
            Command::Shortcuts,
            &SHORTCUTS,
            None,
            icons::KEYBOARD,
            Some("F1"),
        ),
        command(
            Command::SwitchAccount,
            &strings::SWITCH_ACCOUNT,
            None,
            icons::SWITCH,
            None,
        ),
        command(
            Command::SignOut,
            &strings::LOG_OUT,
            None,
            icons::LOGOUT,
            None,
        ),
    ]);
    items.extend(
        NavGroup::ALL
            .into_iter()
            .flat_map(|group| group.routes())
            .filter(|route| !route.is_available())
            .map(|route| go(*route, Section::Soon)),
    );
    items
}

/// The entries that match `query`, by section and then best match first (ties keep the display
/// order). An empty query keeps everything.
fn visible(items: Vec<Item>, query: &str) -> Vec<Item> {
    let mut scored: Vec<(i32, Item)> = items
        .into_iter()
        .filter_map(|item| matching::score(query, &item.label).map(|score| (score, item)))
        .collect();
    scored.sort_by_key(|(score, item)| (item.section, -score));
    scored.into_iter().map(|(_, item)| item).collect()
}

pub enum PaletteEvent {
    Run(Command),
    Closed,
}

impl EventEmitter<PaletteEvent> for Palette {}

pub struct Palette {
    input: Entity<InputState>,
    /// Index among the enabled entries of the list.
    selected: usize,
    _subscription: Subscription,
}

impl Palette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t(&PLACEHOLDER)));
        let subscription =
            cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, _, cx| match event {
                    InputEvent::Change => {
                        this.selected = 0;
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => this.run_selected(cx),
                    _ => {}
                },
            );
        Self {
            input,
            selected: 0,
            _subscription: subscription,
        }
    }

    /// Clear the field and focus it.
    pub fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = 0;
        // The language may have changed since the palette was built.
        let placeholder = cx.t(&PLACEHOLDER);
        self.input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx);
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn entries(&self, cx: &App) -> Vec<Item> {
        let query = self.input.read(cx).value().to_string();
        visible(
            items(
                cx.lang(),
                cx.resolved_theme().dark,
                Prefs::global(cx).theme_mode,
            ),
            &query,
        )
    }

    fn run_selected(&mut self, cx: &mut Context<Self>) {
        let entries = self.entries(cx);
        if let Some(item) = entries
            .iter()
            .filter(|item| item.enabled())
            .nth(self.selected)
        {
            cx.emit(PaletteEvent::Run(item.command));
        }
    }
}

impl Render for Palette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.tokens();
        let entries = self.entries(cx);
        let count = entries.iter().filter(|item| item.enabled()).count();
        let selected = self.selected.min(count.saturating_sub(1));

        let mut rows: Vec<AnyElement> = Vec::new();
        let mut section = None;
        let mut enabled_index = 0;
        for (index, item) in entries.iter().enumerate() {
            if section != Some(item.section) {
                section = Some(item.section);
                rows.push(
                    div()
                        .px(px(10.0))
                        .pt(px(10.0))
                        .pb(px(4.0))
                        .font_family(FONT_MONO)
                        .text_size(px(11.0))
                        .text_color(t.on_surface_variant)
                        .child(cx.t(item.section.title()))
                        .into_any_element(),
                );
            }
            let on = item.enabled() && enabled_index == selected;
            let command = item.command;
            let enabled = item.enabled();
            if enabled {
                enabled_index += 1;
            }
            rows.push(
                div()
                    .id(("palette-row", index))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .min_h(px(if item.subtitle.is_some() { 48.0 } else { 36.0 }))
                    .px(px(10.0))
                    .rounded(px(8.0))
                    .text_size(px(13.5))
                    .text_color(if on {
                        t.on_secondary_container
                    } else if enabled {
                        t.on_surface
                    } else {
                        t.on_surface_variant
                    })
                    .when(on, |d| d.bg(t.secondary_container))
                    .when(enabled, |d| {
                        d.cursor_pointer()
                            .when(!on, |d| d.hover(|s| s.bg(t.surface_container_highest)))
                            .on_click(
                                cx.listener(move |_, _, _, cx| cx.emit(PaletteEvent::Run(command))),
                            )
                    })
                    .child(assets::icon(item.icon, 20.0, false))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(item.label.clone())
                            .children(item.subtitle.clone().map(|subtitle| {
                                div()
                                    .text_size(px(12.0))
                                    .text_color(t.on_surface_variant)
                                    .child(subtitle)
                            })),
                    )
                    .children(item.shortcut.map(|keys| Kbd::new(keys.split(' '))))
                    .when(!enabled, |d| {
                        d.child(
                            div()
                                .font_family(FONT_MONO)
                                .text_size(px(11.0))
                                .child(cx.t(&strings::SOON)),
                        )
                    })
                    .into_any_element(),
            );
        }

        let hint = |keys: &'static str, text: &Text| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(Kbd::new([keys]))
                .child(cx.t(text))
        };
        // The scrim closes the palette; the card eats the clicks that land on it.
        div()
            .id("palette-scrim")
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(96.0))
            .bg(t.scrim.opacity(0.32))
            .occlude()
            .on_click(cx.listener(|_, _, _, cx| cx.emit(PaletteEvent::Closed)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "down" if count > 0 => this.selected = (this.selected + 1).min(count - 1),
                    "up" => this.selected = this.selected.saturating_sub(1),
                    "escape" => cx.emit(PaletteEvent::Closed),
                    _ => return,
                }
                cx.notify();
            }))
            .child(
                div()
                    .id("palette-card")
                    .w(px(600.0))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .p(px(8.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(t.outline_variant)
                    .bg(t.surface_container_high)
                    .font_family(FONT_UI)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .h(px(40.0))
                            .px(px(10.0))
                            .rounded(px(8.0))
                            .bg(t.surface_container_highest)
                            .text_color(t.on_surface_variant)
                            .child(assets::icon(icons::SEARCH, 20.0, false))
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.input).appearance(false).bordered(false),
                                ),
                            )
                            .child(Kbd::new(["Esc"])),
                    )
                    .child(
                        div()
                            .id("palette-list")
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .max_h(px(420.0))
                            .overflow_y_scroll()
                            .children(rows),
                    )
                    .when(entries.is_empty(), |d| {
                        d.child(
                            div()
                                .h(px(36.0))
                                .px(px(10.0))
                                .flex()
                                .items_center()
                                .text_size(px(13.0))
                                .text_color(t.on_surface_variant)
                                .child(cx.t(&NO_RESULTS)),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(14.0))
                            .px(px(10.0))
                            .pt(px(8.0))
                            .border_t_1()
                            .border_color(t.outline_variant)
                            .text_size(px(12.0))
                            .text_color(t.on_surface_variant)
                            .child(hint("↑↓", &HINT_MOVE))
                            .child(hint("Enter", &HINT_OPEN))
                            .child(hint("Esc", &HINT_CLOSE)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Item> {
        items(Lang::Pl, false, ThemeMode::System)
    }

    fn labels(items: &[Item]) -> Vec<&str> {
        items.iter().map(|item| item.label.as_str()).collect()
    }

    #[test]
    fn the_empty_query_lists_every_entry_in_display_order() {
        let list = visible(all(), "");
        assert_eq!(list.len(), all().len());
        assert_eq!(
            labels(&list)[..5],
            ["Dziś", "Plan", "Oceny", "Sprawdziany", "Ustawienia"]
        );
        let sections: Vec<_> = list.iter().map(|item| item.section).collect();
        assert!(sections.is_sorted());
    }

    #[test]
    fn unavailable_screens_are_listed_last_and_disabled() {
        let list = all();
        let soon: Vec<&Item> = list.iter().filter(|item| !item.enabled()).collect();
        assert_eq!(soon.len(), 7);
        assert!(
            soon.iter()
                .all(|item| matches!(item.command, Command::Go(r) if !r.is_available()))
        );
        assert!(list.iter().rev().take(7).all(|item| !item.enabled()));
        assert!(
            list.iter()
                .filter(|item| item.enabled())
                .all(|item| { !matches!(item.command, Command::Go(r) if !r.is_available()) })
        );
    }

    #[test]
    fn the_theme_command_names_the_theme_it_switches_to() {
        let light = items(Lang::Pl, false, ThemeMode::System);
        let dark = items(Lang::En, true, ThemeMode::Dark);
        let find = |list: &[Item]| {
            list.iter()
                .find(|item| item.command == Command::ToggleTheme)
                .cloned()
                .unwrap()
        };
        assert_eq!(find(&light).label, "Przełącz motyw: ciemny");
        assert_eq!(
            find(&light).subtitle.as_deref(),
            Some("Teraz: zgodnie z\u{a0}systemem")
        );
        assert_eq!(find(&dark).label, "Switch theme: light");
        assert_eq!(find(&dark).subtitle.as_deref(), Some("Now: dark"));
    }

    #[test]
    fn typing_without_diacritics_finds_polish_names() {
        assert_eq!(labels(&visible(all(), "dzis")), ["Dziś"]);
        let list = visible(all(), "platnosci");
        assert_eq!(labels(&list), ["Płatności"]);
        assert!(!list[0].enabled());
        assert_eq!(labels(&visible(all(), "SPRAWDZ")), ["Sprawdziany"]);
    }

    #[test]
    fn a_closer_match_comes_first_within_a_section() {
        let list = visible(all(), "sy");
        let at = |label: &str| list.iter().position(|item| item.label == label).unwrap();
        // Both are commands: the one that starts with "sy" beats the one with scattered letters.
        assert!(at("Synchronizuj teraz") < at("Skróty klawiszowe"));
        // The screen "Sprawdziany" still comes first: sections keep their order.
        assert!(at("Sprawdziany") < at("Synchronizuj teraz"));
    }

    #[test]
    fn nothing_matches_gibberish() {
        assert!(visible(all(), "qqqq").is_empty());
    }

    #[test]
    fn english_labels_follow_the_language() {
        let list = visible(items(Lang::En, false, ThemeMode::System), "grad");
        assert_eq!(labels(&list), ["Grades"]);
    }

    #[test]
    fn shortcuts_shown_in_the_list_are_the_real_ones() {
        let list = all();
        let shortcut = |label: &str| {
            list.iter()
                .find(|item| item.label == label)
                .and_then(|item| item.shortcut)
        };
        assert_eq!(shortcut("Dziś"), Some("Ctrl 1"));
        assert_eq!(shortcut("Ustawienia"), Some("Ctrl ,"));
        assert_eq!(shortcut("Synchronizuj teraz"), Some("Ctrl R"));
        assert_eq!(shortcut("Skróty klawiszowe"), Some("F1"));
    }
}
