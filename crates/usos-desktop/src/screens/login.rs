//! Sign-in (SYSTEM.md section 11, d-01 and d-17): choose a university, sign in in the browser,
//! optionally finish with a PIN, and the failure states. The OAuth work happens in
//! `state::login`; this view only shows its progress and sends commands.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, InteractiveElement,
    IntoElement, ParentElement, Render, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Window, div, px,
};
use gpui_component::input::{Input, InputEvent, InputState};
use usos_core::auth::login::CallbackMode;

use crate::assets::{self, icons};
use crate::chrome::{self, Side};
use crate::components::{Banner, BannerKind, Btn, BtnStyle};
use crate::i18n::{I18nExt, Lang, Text, strings};
use crate::state::login::{PRESETS, university_name};
use crate::state::{AppState, Auth, LoginCmd, LoginFailure, LoginStatus};
use crate::theme::{FONT_MONO, FONT_UI, ThemeExt, Tokens};

const TITLE: Text = Text::new(
    "Twój plan, oceny i sprawdziany z USOS",
    "Your timetable, grades and tests from USOS",
);
const LEAD: Text = Text::new(
    "Łączymy się bezpośrednio z USOS Twojej uczelni. Hasło wpisujesz tylko na jej stronie logowania.",
    "We connect straight to your university's USOS. You enter your password only on its sign-in page.",
);
const SEARCH_PLACEHOLDER: Text = Text::new(
    "Szukaj: nazwa, miasto lub adres",
    "Search: name, city or address",
);
const NO_MATCH: Text = Text::new(
    "Nie znaleziono. Podaj adres USOS API poniżej.",
    "Nothing found. Enter the USOS API address below.",
);
const ENTER_URL: Text = Text::new(
    "Nie ma uczelni na liście? Podaj adres USOS API",
    "Not on the list? Enter the USOS API address",
);
const USE_LIST: Text = Text::new("Wróć do listy uczelni", "Back to the university list");
const URL_PLACEHOLDER: Text = Text::new(
    "https://usosapi.uczelnia.pl/",
    "https://usosapi.university.edu/",
);
const URL_HELP: Text = Text::new(
    "Uczelnie spoza listy mogą wymagać własnego klucza aplikacji.",
    "Universities that are not on the list may need their own application key.",
);
const INVALID_URL: Text = Text::new(
    "To nie wygląda na adres USOS API.",
    "That does not look like a USOS API address.",
);
const KEY_BUILT_IN: Text = Text::new("Klucz wbudowany", "Key built in");
const KEY_NEEDED: Text = Text::new("Wymaga własnego klucza", "Needs your own key");
const SIGN_IN_BROWSER: Text = Text::new("Zaloguj w przeglądarce", "Sign in with your browser");
const FOOTER: Text = Text::new(
    "Nieoficjalny klient · GPL-3.0-or-later",
    "Unofficial client · GPL-3.0-or-later",
);
const FOOTER_URL: &str = "gitlab.com/libreusos/LibreUSOS";
const SESSION_ERROR: Text = Text::new(
    "Nie udało się otworzyć zapisanej sesji.",
    "The saved session could not be opened.",
);
const CONNECTING: Text = Text::new("Łączenie z", "Connecting to");
const COMPLETING: Text = Text::new("Kończymy logowanie…", "Finishing sign-in…");
const WAITING_TITLE: Text = Text::new(
    "Dokończ logowanie w przeglądarce",
    "Finish signing in in your browser",
);
const WAITING_LEAD: Text = Text::new(
    "Otworzyliśmy stronę logowania w domyślnej przeglądarce. Po zatwierdzeniu wrócisz tu sam.",
    "We opened the sign-in page in your default browser. After you approve, you come back here on your own.",
);
const STEP_SIGN_IN: Text = Text::new(
    "Zaloguj się kontem uczelnianym",
    "Sign in with your university account",
);
const STEP_ALLOW: Text = Text::new("Zezwól LibreUSOS na dostęp", "Allow LibreUSOS access");
const STEP_ALLOW_SUB: Text = Text::new(
    "Plan, oceny, sprawdziany, adres e-mail",
    "Timetable, grades, tests, email address",
);
const STEP_RETURN: Text = Text::new("Wróć do aplikacji", "Come back to the app");
const STEP_RETURN_LOOPBACK: Text = Text::new(
    "Przeglądarka przekaże odpowiedź na 127.0.0.1",
    "The browser hands the answer to 127.0.0.1",
);
const STEP_RETURN_PIN: Text = Text::new("Wpisz kod PIN poniżej", "Enter the PIN below");
const STUCK: Text = Text::new("Nie wróciło samo?", "Did not come back on its own?");
const STUCK_PIN: Text = Text::new(
    "Wpisz kod PIN, który USOS pokazał po zatwierdzeniu.",
    "Enter the PIN that USOS showed after you approved.",
);
const STUCK_LOOPBACK: Text = Text::new(
    "Zacznie logowanie od nowa w trybie PIN: USOS pokaże kod do wpisania tutaj.",
    "This restarts sign-in in PIN mode: USOS shows a code to enter here.",
);
const USE_PIN: Text = Text::new("Użyj kodu PIN", "Use a PIN code");
const PIN_PLACEHOLDER: Text = Text::new("PIN, np. 52917384", "PIN, e.g. 52917384");
const REOPEN: Text = Text::new("Otwórz ponownie", "Open again");
const COPY_LINK: Text = Text::new("Kopiuj adres", "Copy link");
const COPIED: Text = Text::new("Skopiowano", "Copied");
const EXPIRES: Text = Text::new(
    "Czekamy 10 minut; potem link wygasa i trzeba zacząć od nowa.",
    "We wait 10 minutes; then the link expires and you start over.",
);
const BACK: Text = Text::new("Wróć", "Back");

// Failure copy: (title, body).
const CANCELLED: (Text, Text) = (
    Text::new("Logowanie anulowane", "Sign-in cancelled"),
    Text::new("Nic się nie zmieniło.", "Nothing was changed."),
);
const TIMED_OUT: (Text, Text) = (
    Text::new("Czas na logowanie minął", "Sign-in timed out"),
    Text::new(
        "Nie dostaliśmy odpowiedzi z przeglądarki w ciągu 10 minut.",
        "We got no answer from the browser within 10 minutes.",
    ),
);
const DENIED: (Text, Text) = (
    Text::new("Dostęp nie został przyznany", "Access was not granted"),
    Text::new(
        "Odmówiono dostępu albo strona nie zwróciła kodu.",
        "Access was refused, or the page returned no code.",
    ),
);
const NEEDS_KEY: (Text, Text) = (
    Text::new(
        "Ta uczelnia wymaga klucza aplikacji",
        "This university needs an application key",
    ),
    Text::new(
        "Ta wersja LibreUSOS nie ma klucza dla wybranej uczelni.",
        "This build of LibreUSOS has no key for the chosen university.",
    ),
);
const NO_NETWORK: (Text, Text) = (
    Text::new("Brak połączenia z USOS", "Cannot reach USOS"),
    Text::new(
        "Sprawdź połączenie z internetem i spróbuj ponownie.",
        "Check your internet connection and try again.",
    ),
);
const OTHER: (Text, Text) = (
    Text::new("Nie udało się zalogować", "Sign-in failed"),
    Text::new("Coś poszło nie tak.", "Something went wrong."),
);

pub struct LoginScreen {
    search: Entity<InputState>,
    url: Entity<InputState>,
    pin: Entity<InputState>,
    /// The chosen preset's base URL.
    selected: Option<String>,
    /// The URL entry is shown instead of the list.
    custom: bool,
    show_details: bool,
    copied: bool,
    /// The `preselect_url` already applied, so a later change is noticed.
    applied_preselect: Option<String>,
    /// The language the input placeholders were set in.
    applied_lang: Lang,
    _subscriptions: Vec<Subscription>,
}

impl LoginScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(cx.t(&SEARCH_PLACEHOLDER)));
        let url = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t(&URL_PLACEHOLDER)));
        let pin = cx.new(|cx| InputState::new(window, cx).placeholder(cx.t(&PIN_PLACEHOLDER)));
        let subscriptions = vec![
            cx.observe(&AppState::global(cx), |this, _, cx| {
                this.copied = false;
                cx.notify();
            }),
            cx.subscribe_in(&search, window, |_, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &url,
                window,
                |this, _, event: &InputEvent, _, cx| match event {
                    InputEvent::Change => cx.notify(),
                    InputEvent::PressEnter { .. } => this.submit(cx),
                    _ => {}
                },
            ),
            cx.subscribe_in(&pin, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.submit_pin(cx);
                }
            }),
        ];
        Self {
            search,
            url,
            pin,
            selected: (PRESETS.len() == 1).then(|| PRESETS[0].base_url.to_owned()),
            custom: false,
            show_details: false,
            copied: false,
            applied_preselect: None,
            applied_lang: cx.lang(),
            _subscriptions: subscriptions,
        }
    }

    fn custom_url(&self, cx: &App) -> String {
        self.url.read(cx).value().trim().to_owned()
    }

    fn custom_url_valid(&self, cx: &App) -> bool {
        usos_core::normalize_base_url(&self.custom_url(cx)).is_ok()
    }

    /// The installation the form currently points at, if it is complete.
    fn target(&self, cx: &App) -> Option<String> {
        if self.custom {
            self.custom_url_valid(cx).then(|| self.custom_url(cx))
        } else {
            self.selected.clone()
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if let Some(url) = self.target(cx) {
            AppState::global(cx).update(cx, |state, cx| state.begin_login(url, cx));
        }
    }

    fn submit_pin(&mut self, cx: &mut Context<Self>) {
        let pin = self.pin.read(cx).value().trim().to_owned();
        if !pin.is_empty() {
            AppState::global(cx).update(cx, |state, _| state.login_command(LoginCmd::Pin(pin)));
        }
    }

    fn retry(base_url: String, cx: &mut App) {
        AppState::global(cx).update(cx, |state, cx| state.begin_login(base_url, cx));
    }

    fn leave(cx: &mut App) {
        AppState::global(cx).update(cx, |state, cx| state.reset_login(cx));
    }
}

impl Render for LoginScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.tokens();
        let (login, preselect, error) = {
            let state = AppState::global(cx).read(cx);
            let error = match &state.auth {
                Auth::SignedOut { error } => error.clone(),
                _ => None,
            };
            (state.login.clone(), state.preselect_url.clone(), error)
        };
        if cx.lang() != self.applied_lang {
            self.applied_lang = cx.lang();
            let placeholders = [
                (&self.search, cx.t(&SEARCH_PLACEHOLDER)),
                (&self.url, cx.t(&URL_PLACEHOLDER)),
                (&self.pin, cx.t(&PIN_PLACEHOLDER)),
            ];
            for (input, text) in placeholders {
                input.update(cx, |input, cx| input.set_placeholder(text, window, cx));
            }
        }
        if preselect != self.applied_preselect {
            if let Some(url) = &preselect {
                self.selected = Some(url.clone());
            }
            self.applied_preselect = preselect;
        }

        let card = match login {
            LoginStatus::Idle => self.form(error, &t, cx),
            LoginStatus::Starting { base_url } => self.progress_card(&base_url, true, &t, cx),
            LoginStatus::Completing { base_url } => self.progress_card(&base_url, false, &t, cx),
            LoginStatus::Waiting {
                base_url,
                authorize_url,
                mode,
                ..
            } => self.waiting_card(base_url, authorize_url, mode, &t, cx),
            LoginStatus::Failed { base_url, failure } => {
                self.failed_card(base_url, failure, &t, cx)
            }
        };

        let drag = chrome::drag_area(
            div()
                .id("login-drag")
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(px(52.0)),
            window,
        );
        let close = chrome::close_button(window, cx).map(|button| {
            let holder = div().absolute().top(px(10.0));
            match chrome::close_side(cx) {
                Side::Left => holder.left(px(10.0)),
                Side::Right => holder.right(px(10.0)),
            }
            .child(button)
        });

        div()
            .size_full()
            .relative()
            .bg(t.surface)
            .text_color(t.on_surface)
            .font_family(FONT_UI)
            .child(drag)
            .child(
                div()
                    .id("login-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .min_h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(16.0))
                            .py(px(56.0))
                            .child(card),
                    ),
            )
            .children(close)
    }
}

/// The card: a 1 px ring, a 4 px gap, then the card proper (d-01).
#[derive(IntoElement)]
struct Card {
    inner: gpui::Div,
    ring: gpui::Hsla,
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .w(px(520.0))
            .max_w_full()
            .p(px(4.0))
            .rounded(px(28.0))
            .border_1()
            .border_color(self.ring)
            .child(self.inner)
    }
}

fn host_of(url: &str) -> String {
    usos_core::normalize_base_url(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned())
}

impl LoginScreen {
    fn card(&self, t: &Tokens) -> Card {
        Card {
            ring: t.outline_variant,
            inner: div()
                .flex()
                .flex_col()
                .gap(px(14.0))
                .p(px(28.0))
                .rounded(px(24.0))
                .border_1()
                .border_color(t.outline_variant)
                .bg(t.surface_container_lowest),
        }
    }

    /// The logo with the app name, then the title and the lead.
    fn heading(&self, title: SharedString, lead: Option<SharedString>, t: &Tokens) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .pb(px(14.0))
                    .child(div().text_color(t.primary).child(assets::logo(24.0)))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(strings::APP_NAME),
                    ),
            )
            .child(
                div()
                    .text_size(px(24.0))
                    .line_height(px(30.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(title),
            )
            .when_some(lead, |d, lead| {
                d.child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(21.0))
                        .text_color(t.on_surface_variant)
                        .child(lead),
                )
            })
            .into_any_element()
    }

    /// A text field: the input inside a filled, rounded box.
    fn field(
        &self,
        input: &Entity<InputState>,
        icon: Option<&'static str>,
        t: &Tokens,
    ) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .h(px(40.0))
            .px(px(12.0))
            .rounded(px(8.0))
            .bg(t.surface_container_high)
            .text_color(t.on_surface_variant)
            .children(icon.map(|name| assets::icon(name, 20.0, false)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(t.on_surface)
                    .child(Input::new(input).appearance(false).bordered(false)),
            )
            .into_any_element()
    }

    fn footer(&self, t: &Tokens, cx: &App) -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .justify_center()
            .gap(px(16.0))
            .pt(px(4.0))
            .text_size(px(12.0))
            .text_color(t.on_surface_variant)
            .child(cx.t(&FOOTER))
            .child(FOOTER_URL)
            .into_any_element()
    }

    /// d-01: pick a university and start.
    fn form(&mut self, error: Option<String>, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let t = *t;
        let query = self.search.read(cx).value().trim().to_lowercase();
        let matches: Vec<_> = PRESETS
            .iter()
            .filter(|preset| {
                query.is_empty()
                    || preset.name.to_lowercase().contains(&query)
                    || preset.base_url.contains(&query)
            })
            .collect();
        let ready = self.target(cx).is_some();
        let url_text = self.custom_url(cx);
        let url_invalid = self.custom && !url_text.is_empty() && !self.custom_url_valid(cx);

        let rows = matches.len();
        let list = div()
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .border_1()
            .border_color(t.outline_variant)
            .overflow_hidden()
            .children(matches.iter().enumerate().map(|(index, preset)| {
                let active = self.selected.as_deref() == Some(preset.base_url);
                let base_url = preset.base_url.to_owned();
                let note = if crate::state::login::has_consumer_key(preset.base_url) {
                    &KEY_BUILT_IN
                } else {
                    &KEY_NEEDED
                };
                div()
                    .id(("preset", index))
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .h(px(58.0))
                    .px(px(14.0))
                    .cursor_pointer()
                    .tab_stop(true)
                    .focus_visible(|s| s.border_2().border_color(t.primary))
                    .when(index + 1 < rows, |d| {
                        d.border_b_1().border_color(t.outline_variant)
                    })
                    .hover(|s| s.bg(t.surface_container_low))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = Some(base_url.clone());
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .size(px(20.0))
                            .rounded_full()
                            .border_2()
                            .border_color(if active {
                                t.primary
                            } else {
                                t.on_surface_variant
                            })
                            .when(active, |d| {
                                d.child(div().size(px(10.0)).rounded_full().bg(t.primary))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(14.0)).child(preset.name))
                            .child(
                                div()
                                    .font_family(FONT_MONO)
                                    .text_size(px(11.0))
                                    .text_color(t.on_surface_variant)
                                    .child(format!(
                                        "{} · {}",
                                        host_of(preset.base_url),
                                        cx.t(note)
                                    )),
                            ),
                    )
            }))
            .when(matches.is_empty(), |d| {
                d.child(
                    div()
                        .p(px(14.0))
                        .text_size(px(13.0))
                        .text_color(t.on_surface_variant)
                        .child(cx.t(&NO_MATCH)),
                )
            });

        let chooser = if self.custom {
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(self.field(&self.url, Some(icons::LINK), &t))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(if url_invalid {
                            t.error
                        } else {
                            t.on_surface_variant
                        })
                        .child(if url_invalid {
                            cx.t(&INVALID_URL)
                        } else {
                            cx.t(&URL_HELP)
                        }),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(self.field(&self.search, Some(icons::SEARCH), &t))
                .child(list)
                .into_any_element()
        };

        let toggle_label = if self.custom { &USE_LIST } else { &ENTER_URL };
        self.card(&t)
            .child(self.heading(cx.t(&TITLE), Some(cx.t(&LEAD)), &t))
            .when_some(error, |d, error| {
                d.child(
                    Banner::new(BannerKind::Error, icons::ERROR, cx.t(&SESSION_ERROR))
                        .detail(error),
                )
            })
            .child(chooser)
            .child(
                div()
                    .id("login-toggle-custom")
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(13.0))
                    .text_color(t.primary)
                    .cursor_pointer()
                    .tab_stop(true)
                    .focus_visible(|s| s.border_2().border_color(t.primary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.custom = !this.custom;
                        cx.notify();
                    }))
                    .child(assets::icon(icons::LINK, 18.0, false))
                    .child(cx.t(toggle_label)),
            )
            .child(
                Btn::new("login-start", BtnStyle::Filled)
                    .wide()
                    .icon(icons::OPEN_EXTERNAL)
                    .label(cx.t(&SIGN_IN_BROWSER))
                    .disabled(!ready)
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
            )
            .child(self.footer(&t, cx))
            .into_any_element()
    }

    /// "Connecting to ..." and "Finishing sign-in...".
    fn progress_card(
        &mut self,
        base_url: &str,
        connecting: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let text = if connecting {
            format!("{} {}…", cx.t(&CONNECTING), university_name(base_url)).into()
        } else {
            cx.t(&COMPLETING)
        };
        self.card(t)
            .child(self.heading(text, None, t))
            .child(
                div().flex().justify_end().child(
                    Btn::new("login-progress-cancel", BtnStyle::Ghost)
                        .label(cx.t(&strings::CANCEL))
                        .on_click(|_, _, cx| Self::leave(cx)),
                ),
            )
            .into_any_element()
    }

    /// d-17: the browser is open; steps, PIN fallback and the actions.
    fn waiting_card(
        &mut self,
        _base_url: String,
        authorize_url: String,
        mode: CallbackMode,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let pin_mode = mode == CallbackMode::Oob;
        let step = |number: usize, title: SharedString, sub: SharedString, current: bool| {
            div()
                .flex()
                .items_start()
                .gap(px(14.0))
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(px(28.0))
                        .rounded_full()
                        .border_2()
                        .border_color(if current {
                            t.primary
                        } else {
                            t.outline_variant
                        })
                        .text_color(if current {
                            t.primary
                        } else {
                            t.on_surface_variant
                        })
                        .font_family(FONT_MONO)
                        .text_size(px(12.0))
                        .child(number.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if current {
                                    t.on_surface
                                } else {
                                    t.on_surface_variant
                                })
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(t.on_surface_variant)
                                .child(sub),
                        ),
                )
        };
        let return_sub = if pin_mode {
            &STEP_RETURN_PIN
        } else {
            &STEP_RETURN_LOOPBACK
        };

        let copy_url = authorize_url.clone();
        let host: SharedString = host_of(&authorize_url).into();
        let open_url = authorize_url;
        let copied = self.copied;

        let stuck = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .pt(px(14.0))
            .border_t_1()
            .border_color(t.outline_variant)
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(cx.t(&STUCK)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(t.on_surface_variant)
                    .child(cx.t(if pin_mode {
                        &STUCK_PIN
                    } else {
                        &STUCK_LOOPBACK
                    })),
            )
            .child(if pin_mode {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().flex_1().child(self.field(&self.pin, None, &t)))
                    .child(
                        Btn::new("login-pin-confirm", BtnStyle::Filled)
                            .label(cx.t(&strings::CONFIRM))
                            .on_click(cx.listener(|this, _, _, cx| this.submit_pin(cx))),
                    )
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .child(
                        Btn::new("login-use-pin", BtnStyle::Outlined)
                            .label(cx.t(&USE_PIN))
                            .on_click(|_, _, cx| {
                                AppState::global(cx)
                                    .update(cx, |state, _| state.login_command(LoginCmd::UsePin));
                            }),
                    )
                    .into_any_element()
            });

        self.card(&t)
            .child(self.heading(cx.t(&WAITING_TITLE), Some(cx.t(&WAITING_LEAD)), &t))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .py(px(6.0))
                    .child(step(1, cx.t(&STEP_SIGN_IN), host, true))
                    .child(step(2, cx.t(&STEP_ALLOW), cx.t(&STEP_ALLOW_SUB), false))
                    .child(step(3, cx.t(&STEP_RETURN), cx.t(return_sub), false)),
            )
            .child(stuck)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Btn::new("login-reopen", BtnStyle::Outlined)
                            .icon(icons::OPEN_EXTERNAL)
                            .label(cx.t(&REOPEN))
                            .on_click(move |_, _, cx| cx.open_url(&open_url)),
                    )
                    .child(
                        Btn::new("login-copy", BtnStyle::Outlined)
                            .icon(if copied { icons::CHECK } else { icons::COPY })
                            .label(cx.t(if copied { &COPIED } else { &COPY_LINK }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copy_url.clone()));
                                this.copied = true;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Btn::new("login-cancel", BtnStyle::Ghost)
                            .label(cx.t(&strings::CANCEL))
                            .on_click(|_, _, cx| Self::leave(cx)),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(t.on_surface_variant)
                    .child(cx.t(&EXPIRES)),
            )
            .into_any_element()
    }

    /// Every way a login ends without success.
    fn failed_card(
        &mut self,
        base_url: String,
        failure: LoginFailure,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let (copy, details) = match &failure {
            LoginFailure::Cancelled => (&CANCELLED, None),
            LoginFailure::Timeout => (&TIMED_OUT, None),
            LoginFailure::Denied => (&DENIED, None),
            LoginFailure::NeedsKey => (&NEEDS_KEY, None),
            LoginFailure::Network => (&NO_NETWORK, None),
            LoginFailure::Other(text) => (&OTHER, Some(text.clone())),
        };
        // Retrying cannot help when the build has no key for this university.
        let can_retry = failure != LoginFailure::NeedsKey;
        let show_details = self.show_details;
        self.card(&t)
            .child(self.heading(cx.t(&copy.0), Some(cx.t(&copy.1)), &t))
            .when_some(details, |d, details| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div().flex().child(
                                Btn::new("login-details", BtnStyle::Ghost)
                                    .icon(if show_details {
                                        icons::EXPAND_MORE
                                    } else {
                                        icons::CHEVRON_RIGHT
                                    })
                                    .label(cx.t(&strings::DETAILS))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_details = !this.show_details;
                                        cx.notify();
                                    })),
                            ),
                        )
                        .when(show_details, |d| {
                            d.child(
                                div()
                                    .p(px(10.0))
                                    .rounded(px(8.0))
                                    .bg(t.surface_container_highest)
                                    .font_family(FONT_MONO)
                                    .text_size(px(12.0))
                                    .text_color(t.on_surface_variant)
                                    .child(details),
                            )
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Btn::new("login-back", BtnStyle::Ghost)
                            .label(cx.t(&BACK))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_details = false;
                                Self::leave(cx);
                            })),
                    )
                    .when(can_retry, |d| {
                        d.child(
                            Btn::new("login-retry", BtnStyle::Filled)
                                .label(cx.t(&strings::TRY_AGAIN))
                                .on_click(move |_, _, cx| Self::retry(base_url.clone(), cx)),
                        )
                    }),
            )
            .into_any_element()
    }
}
