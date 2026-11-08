//! The keyboard shortcuts: the list Settings shows (SYSTEM.md section 15) and the bindings for
//! the keys that only open that list. The other bindings live in `main.rs` next to their actions.

use gpui::prelude::FluentBuilder as _;
use gpui::{App, KeyBinding, ParentElement, SharedString, Styled, div, px};

use crate::components::Kbd;
use crate::i18n::{I18nExt, Text};
use crate::nav::OpenShortcuts;
use crate::theme::ThemeExt;

/// One line of the list. `served` is false for a key whose screen does not handle it yet; the
/// caps are dimmed then.
struct Shortcut {
    label: Text,
    keys: &'static [&'static str],
    served: bool,
}

const fn key(label: Text, keys: &'static [&'static str]) -> Shortcut {
    Shortcut {
        label,
        keys,
        served: true,
    }
}

const fn pending(label: Text, keys: &'static [&'static str]) -> Shortcut {
    Shortcut {
        label,
        keys,
        served: false,
    }
}

struct Group {
    title: Text,
    shortcuts: &'static [Shortcut],
}

const GROUPS: [Group; 3] = [
    Group {
        title: Text::new("Nawigacja", "Navigation"),
        shortcuts: &[
            key(
                Text::new(
                    "Dziś, Plan, Oceny, Sprawdziany",
                    "Today, Timetable, Grades, Tests",
                ),
                &["Ctrl", "1…4"],
            ),
            key(
                Text::new("Szukaj i polecenia", "Search and commands"),
                &["Ctrl", "K"],
            ),
            key(Text::new("Ustawienia", "Settings"), &["Ctrl", ","]),
            key(Text::new("Wstecz", "Back"), &["Alt", "←"]),
            key(Text::new("Dalej", "Forward"), &["Alt", "→"]),
            key(
                Text::new("Zamknij panel lub paletę", "Close a panel or the palette"),
                &["Esc"],
            ),
        ],
    },
    Group {
        title: Text::new("Plan", "Timetable"),
        shortcuts: &[
            pending(Text::new("Dziś", "Today"), &["T"]),
            pending(
                Text::new(
                    "Poprzedni, następny dzień lub tydzień",
                    "Previous or next day or week",
                ),
                &["←", "→"],
            ),
            pending(
                Text::new("Widok dnia / tygodnia", "Day / week view"),
                &["D / W"],
            ),
            pending(
                Text::new(
                    "Szczegóły zaznaczonych zajęć",
                    "Details of the selected class",
                ),
                &["Enter"],
            ),
        ],
    },
    Group {
        title: Text::new("Ogólne", "General"),
        shortcuts: &[
            key(Text::new("Synchronizuj teraz", "Sync now"), &["Ctrl", "R"]),
            key(Text::new("Zakończ", "Quit"), &["Ctrl", "Q"]),
            key(Text::new("Ta lista", "This list"), &["F1"]),
        ],
    },
];

const NOTE: Text = Text::new(
    "Na macOS zamiast Ctrl jest Cmd, a zamiast Alt jest Option. Skróty jednoklawiszowe działają, gdy kursor nie stoi w polu tekstowym.",
    "On macOS, Cmd replaces Ctrl and Option replaces Alt. Single-key shortcuts work when the cursor is not in a text field.",
);

/// F1 and Ctrl / open the list anywhere; `?` only where no text field can be typing (the shell
/// sets the "Browsing" context then).
pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("f1", OpenShortcuts, None),
        KeyBinding::new("ctrl-/", OpenShortcuts, None),
        KeyBinding::new("shift-/", OpenShortcuts, Some("Browsing")),
    ]);
}

/// The Settings section: the note, then one card per group.
pub fn view(cx: &App) -> gpui::Div {
    let t = *cx.tokens();
    let mut body = div().flex().flex_col().gap(px(24.0)).child(
        div()
            .text_size(px(12.5))
            .text_color(t.on_surface_variant)
            .child(cx.t(&NOTE)),
    );
    for group in &GROUPS {
        let title: SharedString = cx.t(&group.title);
        let rows = group.shortcuts.iter().enumerate().map(|(index, shortcut)| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(16.0))
                .h(px(40.0))
                .px(px(16.0))
                .border_color(t.outline_variant)
                .when(index > 0, |d| d.border_t_1())
                .text_size(px(13.5))
                .child(cx.t(&shortcut.label))
                .child(Kbd::new(shortcut.keys.iter().copied()).muted(!shortcut.served))
        });
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .rounded(px(12.0))
                        .border_1()
                        .border_color(t.outline_variant)
                        .overflow_hidden()
                        .children(rows),
                ),
        );
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_has_shortcuts_and_every_shortcut_has_keys() {
        for group in &GROUPS {
            assert!(!group.shortcuts.is_empty());
            for shortcut in group.shortcuts {
                assert!(!shortcut.keys.is_empty());
                assert!(!shortcut.label.pl.is_empty() && !shortcut.label.en.is_empty());
            }
        }
    }

    #[test]
    fn only_the_plan_keys_wait_for_their_screen() {
        for group in &GROUPS {
            let waiting = group.shortcuts.iter().all(|s| !s.served);
            assert_eq!(waiting, group.title.en == "Timetable", "{}", group.title.en);
        }
    }

    #[test]
    fn the_list_is_the_designed_table() {
        let total: usize = GROUPS.iter().map(|g| g.shortcuts.len()).sum();
        assert_eq!(total, 13);
    }
}
