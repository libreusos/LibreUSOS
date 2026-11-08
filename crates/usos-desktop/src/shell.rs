//! The window's root view: sidebar (or the 60 px rail), the current screen, the account menu,
//! the confirmation dialog and the palette. It chooses between the login screen, the splash
//! and the signed-in layout from the app state, and knows nothing about what a screen shows.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, WindowAppearance, div, px,
};
use gpui_component::tooltip::Tooltip;

use crate::assets::{self, icons};
use crate::chrome::{self, Side};
use crate::components::{Btn, BtnStyle};
use crate::i18n::{I18nExt, Text, strings};
use crate::nav::{
    Back, Forward, GoGrades, GoTests, GoToday, GoWeek, Group, Nav, OpenPalette, OpenSettings,
    OpenShortcuts, Refresh, Route, SignOut, SwitchAccount,
};
use crate::palette::{self, Command, Palette, PaletteEvent};
use crate::screens::settings::{Section, open_section};
use crate::screens::{
    GradesScreen, LoginScreen, SettingsScreen, TestsScreen, TodayScreen, WeekScreen,
};
use crate::state::{AppState, Auth, LoginStatus, SyncStatus};
use crate::theme::{self, FONT_MONO, FONT_UI, ThemeExt, Tokens};

/// Below this window width the sidebar collapses to the icon rail.
const NARROW_BELOW: f32 = 960.0;
const SIDEBAR_WIDTH: f32 = 248.0;
const RAIL_WIDTH: f32 = 60.0;

const SWITCH_TITLE: Text = Text::new(
    "Zmienić uczelnię lub konto?",
    "Change university or account?",
);
const SWITCH_CONFIRM: Text = Text::new("Zmień", "Change");
const MENU_SIGNED_IN: Text = Text::new("Zalogowano jako", "Signed in as");

/// What the confirmation dialog is about to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirm {
    SignOut,
    SwitchAccount,
}

pub struct Shell {
    focus: FocusHandle,
    menu_open: bool,
    confirm: Option<Confirm>,
    palette_open: bool,
    palette: Entity<Palette>,
    today: Entity<TodayScreen>,
    week: Entity<WeekScreen>,
    grades: Entity<GradesScreen>,
    tests: Entity<TestsScreen>,
    settings: Entity<SettingsScreen>,
    login: Entity<LoginScreen>,
    title: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        chrome::request_server_decorations(window);
        let palette = cx.new(|cx| Palette::new(window, cx));
        let subscriptions = vec![
            cx.observe(&Nav::global(cx), |_, _, cx| cx.notify()),
            cx.observe(&AppState::global(cx), |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &palette,
                window,
                |this, _, event: &PaletteEvent, window, cx| match event {
                    PaletteEvent::Run(command) => {
                        this.close_palette(window, cx);
                        this.run(*command, cx);
                    }
                    PaletteEvent::Closed => this.close_palette(window, cx),
                },
            ),
            cx.observe_window_appearance(window, |_, window, cx| {
                theme::set_system_dark(is_dark(window.appearance()), cx);
            }),
            cx.observe_window_bounds(window, |_, _, cx| cx.notify()),
            cx.observe_button_layout_changed(window, |_, _, cx| cx.notify()),
        ];
        theme::set_system_dark(is_dark(window.appearance()), cx);
        Self {
            focus: cx.focus_handle(),
            menu_open: false,
            confirm: None,
            palette_open: false,
            palette,
            today: cx.new(|_| TodayScreen::new()),
            week: cx.new(|_| WeekScreen::new()),
            grades: cx.new(|_| GradesScreen::new()),
            tests: cx.new(|_| TestsScreen::new()),
            settings: cx.new(|_| SettingsScreen::new()),
            login: cx.new(|cx| LoginScreen::new(window, cx)),
            title: SharedString::default(),
            _subscriptions: subscriptions,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    fn signed_in(&self, cx: &App) -> bool {
        matches!(AppState::global(cx).read(cx).auth, Auth::SignedIn { .. })
    }

    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.signed_in(cx) {
            return;
        }
        self.menu_open = false;
        self.palette_open = true;
        self.palette
            .update(cx, |palette, cx| palette.reset(window, cx));
        cx.notify();
    }

    fn run(&mut self, command: Command, cx: &mut Context<Self>) {
        self.menu_open = false;
        match command {
            Command::Go(route) => Nav::go(route, cx),
            Command::Refresh => AppState::global(cx).update(cx, |state, cx| state.sync(cx)),
            Command::ToggleTheme => palette::toggle_theme(cx),
            Command::Shortcuts => open_section(Section::Shortcuts, cx),
            Command::SwitchAccount => self.confirm = Some(Confirm::SwitchAccount),
            Command::SignOut => self.confirm = Some(Confirm::SignOut),
        }
        cx.notify();
    }

    /// Close the topmost overlay, else go back one page.
    fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm.take().is_some() || std::mem::take(&mut self.menu_open) {
            cx.notify();
        } else if self.palette_open {
            self.close_palette(window, cx);
        } else {
            Nav::go_back(cx);
        }
    }

    fn confirmed(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        // Both paths leave the window signed out; "switch" keeps the university selected.
        let preselect = confirm == Confirm::SwitchAccount;
        AppState::global(cx).update(cx, |state, cx| state.logout(preselect, cx));
        cx.notify();
    }

    fn screen(&self, route: Route) -> AnyElement {
        match route {
            Route::Today => self.today.clone().into_any_element(),
            Route::Week => self.week.clone().into_any_element(),
            Route::Grades => self.grades.clone().into_any_element(),
            Route::Tests => self.tests.clone().into_any_element(),
            // The grouped routes are not reachable yet (the sidebar disables them).
            _ => self.settings.clone().into_any_element(),
        }
    }
}

fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

/// The initials in the account avatar: first letters of the first two words.
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.tokens();
        let (auth, login_idle) = {
            let state = AppState::global(cx).read(cx);
            (state.auth.clone(), matches!(state.login, LoginStatus::Idle))
        };
        let route = Nav::global(cx).read(cx).route();
        let title: SharedString = match auth {
            Auth::SignedIn { .. } if login_idle => {
                format!("{} · {}", cx.t(route.label()), strings::APP_NAME).into()
            }
            _ => strings::APP_NAME.into(),
        };
        if title != self.title {
            window.set_window_title(&title);
            self.title = title;
        }

        let content: AnyElement = match auth {
            Auth::Starting => self.splash(window, cx),
            Auth::SignedIn { .. } if login_idle => self.main_layout(route, window, cx),
            _ => self.login.clone().into_any_element(),
        };

        let narrow = window.viewport_size().width < px(NARROW_BELOW);
        let signed_in = matches!(auth, Auth::SignedIn { .. });
        // No text field can be typing: the single-key shortcuts ("?") may fire.
        let browsing = signed_in && login_idle && !self.palette_open;
        let root = div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(t.surface)
            .text_color(t.on_surface)
            .font_family(FONT_UI)
            .key_context(if browsing { "Shell Browsing" } else { "Shell" })
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &GoToday, _, cx| go_if_signed_in(Route::Today, cx)))
            .on_action(cx.listener(|_, _: &GoWeek, _, cx| go_if_signed_in(Route::Week, cx)))
            .on_action(cx.listener(|_, _: &GoGrades, _, cx| go_if_signed_in(Route::Grades, cx)))
            .on_action(cx.listener(|_, _: &GoTests, _, cx| go_if_signed_in(Route::Tests, cx)))
            .on_action(
                cx.listener(|_, _: &OpenSettings, _, cx| go_if_signed_in(Route::Settings, cx)),
            )
            .on_action(
                cx.listener(|this, _: &OpenPalette, window, cx| this.open_palette(window, cx)),
            )
            .on_action(cx.listener(|this, _: &Refresh, _, cx| {
                if this.signed_in(cx) {
                    AppState::global(cx).update(cx, |state, cx| state.sync(cx));
                }
            }))
            .on_action(cx.listener(|this, _: &Back, window, cx| this.back(window, cx)))
            .on_action(cx.listener(|this, _: &Forward, _, cx| {
                if this.signed_in(cx) {
                    Nav::go_forward(cx);
                }
            }))
            .on_action(cx.listener(|this, _: &OpenShortcuts, window, cx| {
                if this.signed_in(cx) {
                    this.close_palette(window, cx);
                    open_section(Section::Shortcuts, cx);
                }
            }))
            .on_action(
                cx.listener(|this, _: &SwitchAccount, _, cx| this.run(Command::SwitchAccount, cx)),
            )
            .on_action(cx.listener(|this, _: &SignOut, _, cx| this.run(Command::SignOut, cx)));
        let mut root = chrome::framed(root, window, cx)
            .child(content)
            .children(chrome::resize_handles(window));

        if signed_in && self.menu_open {
            root = root
                .child(
                    div()
                        .id("menu-scrim")
                        .absolute()
                        .inset_0()
                        .occlude()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.menu_open = false;
                            cx.notify();
                        })),
                )
                .child(self.account_menu(narrow, &t, cx));
        }
        if signed_in && self.confirm.is_some() {
            root = root.child(self.confirm_dialog(&t, cx));
        }
        if signed_in && self.palette_open {
            root = root.child(self.palette.clone());
        }
        root
    }
}

fn go_if_signed_in(route: Route, cx: &mut Context<Shell>) {
    if matches!(AppState::global(cx).read(cx).auth, Auth::SignedIn { .. }) {
        Nav::go(route, cx);
    }
}

// Layout pieces.
impl Shell {
    fn splash(&self, window: &Window, cx: &App) -> AnyElement {
        let t = cx.tokens();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(t.primary)
            .child(assets::logo(56.0))
            .children(chrome::close_button(window, cx).map(|button| {
                div()
                    .absolute()
                    .top(px(10.0))
                    .map_close_side(cx)
                    .child(button)
            }))
            .into_any_element()
    }

    fn main_layout(&self, route: Route, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let narrow = window.viewport_size().width < px(NARROW_BELOW);
        div()
            .size_full()
            .flex()
            .child(self.sidebar(route, narrow, cx))
            .child(div().flex_1().min_w_0().h_full().child(self.screen(route)))
            .into_any_element()
    }

    fn sidebar(&self, route: Route, narrow: bool, cx: &mut Context<Self>) -> AnyElement {
        let t = *cx.tokens();
        let (user, university, term, sync) = {
            let state = AppState::global(cx).read(cx);
            (
                state.user_name(),
                state.university(),
                state.data.term.as_ref().map(|term| term.id.clone()),
                state.sync.clone(),
            )
        };

        let brand = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.0))
            .h(px(52.0))
            .px(px(8.0))
            .when(narrow, |d| d.justify_center().px(px(0.0)))
            .child(div().text_color(t.primary).child(assets::logo(22.0)))
            .when(!narrow, |d| {
                d.child(
                    div()
                        .flex_1()
                        .text_size(px(15.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(strings::APP_NAME),
                )
                .children(term.map(|term| {
                    div()
                        .font_family(FONT_MONO)
                        .text_size(px(11.0))
                        .px(px(5.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(t.outline_variant)
                        .text_color(t.on_surface_variant)
                        .child(term)
                }))
            });

        let search_label = cx.t(&strings::SEARCH);
        let search = {
            let tip = search_label.clone();
            div()
                .id("sidebar-search")
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .h(px(34.0))
                .mx(px(4.0))
                .mb(px(10.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(t.surface_container_highest)
                .text_color(t.on_surface_variant)
                .text_size(px(13.0))
                .cursor_pointer()
                .tab_stop(true)
                .focus_visible(|s| s.border_2().border_color(t.primary))
                .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                .on_click(cx.listener(|this, _, window, cx| this.open_palette(window, cx)))
                .child(assets::icon(icons::SEARCH, 20.0, false))
                .when(!narrow, |d| {
                    d.child(div().flex_1().child(search_label))
                        .child(keycap("Ctrl K", &t))
                })
                .when(narrow, |d| d.justify_center().px(px(0.0)))
        };

        let daily = div()
            .flex()
            .flex_col()
            .children(Route::DAILY.map(|r| nav_item(r, r == route, narrow, &t, cx)));
        let groups = div().flex().flex_col().children(Group::ALL.map(|group| {
            div()
                .flex()
                .flex_col()
                .child(if narrow {
                    div()
                        .h(px(1.0))
                        .mx(px(12.0))
                        .my(px(8.0))
                        .bg(t.outline_variant)
                } else {
                    div()
                        .px(px(10.0))
                        .pt(px(14.0))
                        .pb(px(4.0))
                        .font_family(FONT_MONO)
                        .text_size(px(11.0))
                        .text_color(t.on_surface_variant)
                        .child(cx.t(group.title()).to_uppercase())
                })
                .children(
                    group
                        .routes()
                        .iter()
                        .map(|r| nav_item(*r, false, narrow, &t, cx)),
                )
        }));

        let scroll = div()
            .id("sidebar-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(daily)
            .child(groups);

        let footer = div()
            .flex()
            .flex_none()
            .flex_col()
            .child(nav_item(
                Route::Settings,
                route == Route::Settings,
                narrow,
                &t,
                cx,
            ))
            .child(
                div()
                    .h(px(1.0))
                    .mx(px(4.0))
                    .my(px(6.0))
                    .bg(t.outline_variant),
            )
            .child(self.sync_row(&sync, narrow, &t, cx))
            .child(self.account_row(user, university, narrow, &t, cx));

        div()
            .id("sidebar")
            .flex()
            .flex_none()
            .flex_col()
            .h_full()
            .w(px(if narrow { RAIL_WIDTH } else { SIDEBAR_WIDTH }))
            .px(px(8.0))
            .pb(px(10.0))
            .bg(t.surface_container_low)
            .border_r_1()
            .border_color(t.outline_variant)
            .child(brand)
            .child(search)
            .child(scroll)
            .child(footer)
            .into_any_element()
    }

    fn sync_row(
        &self,
        sync: &SyncStatus,
        narrow: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let (dot, text) = sync_summary(sync, t, cx);
        let busy = *sync == SyncStatus::Syncing;
        let refresh = Btn::new("sync-refresh", BtnStyle::Ghost)
            .icon(icons::REFRESH)
            .tooltip(cx.t(&strings::SYNC_NOW))
            .disabled(busy)
            .on_click(|_, _, cx| {
                AppState::global(cx).update(cx, |state, cx| state.sync(cx));
            });
        let status_tip = text.clone();
        div()
            .id("sync-row")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .h(px(36.0))
            .px(px(if narrow { 0.0 } else { 8.0 }))
            .text_size(px(12.0))
            .text_color(t.on_surface_variant)
            .when(narrow, |d| d.flex_col().h_auto().py(px(4.0)))
            .child(
                div()
                    .id("sync-dot")
                    .flex_none()
                    .size(px(7.0))
                    .rounded_full()
                    .bg(dot)
                    .when(narrow, |d| {
                        d.tooltip(move |window, cx| {
                            Tooltip::new(status_tip.clone()).build(window, cx)
                        })
                    }),
            )
            .when(!narrow, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .child(text),
                )
            })
            .child(refresh)
            .into_any_element()
    }

    fn account_row(
        &self,
        user: Option<String>,
        university: String,
        narrow: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let name = user
            .clone()
            .unwrap_or_else(|| cx.t(&strings::ACCOUNT).to_string());
        let avatar = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(32.0))
            .rounded_full()
            .bg(t.secondary_container)
            .text_color(t.on_secondary_container)
            .font_family(FONT_MONO)
            .text_size(px(12.0))
            .child(user.as_deref().map(initials).unwrap_or_default());
        let tip: SharedString = name.clone().into();
        div()
            .id("account-row")
            .flex()
            .flex_none()
            .items_center()
            .gap(px(10.0))
            .h(px(48.0))
            .px(px(if narrow { 0.0 } else { 8.0 }))
            .rounded(px(8.0))
            .cursor_pointer()
            .tab_stop(true)
            .focus_visible(|s| s.border_2().border_color(t.primary))
            .hover(|s| s.bg(t.surface_container_high))
            .when(narrow, |d| {
                d.justify_center()
                    .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.menu_open = !this.menu_open;
                cx.notify();
            }))
            .child(avatar)
            .when(!narrow, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(t.on_surface_variant)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(university),
                        ),
                )
            })
            .into_any_element()
    }

    fn account_menu(&self, narrow: bool, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let t = *t;
        let name = AppState::global(cx).read(cx).user_name();
        let item = |id: &'static str,
                    icon: &'static str,
                    label: SharedString,
                    danger: bool,
                    command: Command| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(px(10.0))
                .h(px(36.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .text_size(px(13.0))
                .text_color(if danger { t.error } else { t.on_surface })
                .hover(|s| s.bg(t.surface_container_highest))
                .child(assets::icon(icon, 20.0, false))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| this.run(command, cx)))
        };
        div()
            .id("account-menu")
            .absolute()
            .left(px(8.0))
            .bottom(px(60.0))
            .w(px(if narrow { 260.0 } else { SIDEBAR_WIDTH - 16.0 }))
            .p(px(6.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(t.outline_variant)
            .bg(t.surface_container_high)
            .shadow_lg()
            .occlude()
            .flex()
            .flex_col()
            .when_some(name, |d, name| {
                d.child(
                    div()
                        .px(px(10.0))
                        .py(px(6.0))
                        .text_size(px(12.0))
                        .text_color(t.on_surface_variant)
                        .child(format!("{} {}", cx.t(&MENU_SIGNED_IN), name)),
                )
            })
            .child(item(
                "menu-switch",
                icons::SWITCH,
                cx.t(&strings::SWITCH_ACCOUNT),
                false,
                Command::SwitchAccount,
            ))
            .child(item(
                "menu-logout",
                icons::LOGOUT,
                cx.t(&strings::LOG_OUT),
                true,
                Command::SignOut,
            ))
            .into_any_element()
    }

    fn confirm_dialog(&self, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let t = *t;
        let (title, confirm_label) = match self.confirm {
            Some(Confirm::SwitchAccount) => (&SWITCH_TITLE, &SWITCH_CONFIRM),
            _ => (&strings::LOG_OUT_TITLE, &strings::LOG_OUT),
        };
        div()
            .id("confirm-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(t.scrim.opacity(0.32))
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| {
                this.confirm = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("confirm-card")
                    .w(px(400.0))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .p(px(20.0))
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(t.outline_variant)
                    .bg(t.surface_container_high)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(cx.t(title)),
                    )
                    .child(
                        div()
                            .text_size(px(13.5))
                            .text_color(t.on_surface_variant)
                            .child(cx.t(&strings::LOG_OUT_BODY)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.0))
                            .pt(px(12.0))
                            .child(
                                Btn::new("confirm-cancel", BtnStyle::Ghost)
                                    .label(cx.t(&strings::CANCEL))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Btn::new("confirm-ok", BtnStyle::Filled)
                                    .label(cx.t(confirm_label))
                                    .on_click(cx.listener(|this, _, _, cx| this.confirmed(cx))),
                            ),
                    ),
            )
            .into_any_element()
    }
}

/// One sidebar row (or rail icon). Items without data behind them are shown but disabled, with
/// the status word "wkrótce" and a tooltip.
fn nav_item(
    route: Route,
    active: bool,
    narrow: bool,
    t: &Tokens,
    cx: &mut Context<Shell>,
) -> AnyElement {
    let t = *t;
    let enabled = route.is_available();
    let label = cx.t(route.label());
    let soon = cx.t(&strings::SOON);
    let tip: SharedString = match (enabled, narrow) {
        (true, _) => label.clone(),
        (false, true) => format!("{label} · {soon}").into(),
        (false, false) => cx.t(&strings::SOON_TOOLTIP),
    };
    let fg = if active {
        t.on_secondary_container
    } else if enabled {
        t.on_surface_variant
    } else {
        t.on_surface_variant.opacity(0.62)
    };
    let row = div()
        .id(("nav", route as usize))
        .flex()
        .flex_none()
        .items_center()
        .h(px(34.0))
        .my(px(1.0))
        .rounded(px(8.0))
        .text_color(fg)
        .text_size(px(13.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .when(narrow, |d| d.justify_center())
        .when(!narrow, |d| d.gap(px(10.0)).pl(px(10.0)).pr(px(8.0)))
        .when(active, |d| d.bg(t.secondary_container))
        .child(assets::icon(route.icon(), 20.0, active))
        .when(!narrow, |d| {
            d.child(
                div()
                    .flex_1()
                    .whitespace_nowrap()
                    .text_color(if active || !enabled { fg } else { t.on_surface })
                    .child(label),
            )
            .child(
                div()
                    .font_family(FONT_MONO)
                    .text_size(px(11.0))
                    .child(if enabled {
                        route.shortcut().map(SharedString::from).unwrap_or_default()
                    } else {
                        soon
                    }),
            )
        })
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx));
    if enabled {
        row.cursor_pointer()
            .tab_stop(true)
            .focus_visible(|s| s.border_2().border_color(t.primary))
            .when(!active, |d| d.hover(|s| s.bg(t.surface_container_high)))
            .on_click(move |_, _, cx| Nav::go(route, cx))
            .into_any_element()
    } else {
        row.into_any_element()
    }
}

fn keycap(text: &'static str, t: &Tokens) -> impl IntoElement {
    div()
        .font_family(FONT_MONO)
        .text_size(px(11.0))
        .px(px(5.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(t.outline_variant)
        .bg(t.surface)
        .text_color(t.on_surface_variant)
        .child(text)
}

/// The colour of the sync dot and the footer text for a sync status.
fn sync_summary(sync: &SyncStatus, t: Tokens, cx: &App) -> (gpui::Hsla, SharedString) {
    match sync {
        SyncStatus::Idle => (t.outline, cx.t(&strings::SYNC_NEVER)),
        SyncStatus::Syncing => (t.primary, cx.t(&strings::SYNC_SYNCING)),
        SyncStatus::Fresh { at } => (
            t.primary,
            format!("{} {}", cx.t(&strings::SYNC_FRESH), at.format("%H:%M")).into(),
        ),
        SyncStatus::Offline { cached_at } => {
            let text = match cached_at {
                Some(at) => format!(
                    "{} · {}",
                    cx.t(&strings::SYNC_OFFLINE),
                    at.with_timezone(&chrono::Local).format("%d.%m %H:%M")
                ),
                None => cx.t(&strings::SYNC_OFFLINE).to_string(),
            };
            (t.on_surface_variant, text.into())
        }
        SyncStatus::Expired => (t.error, cx.t(&strings::SYNC_EXPIRED)),
        SyncStatus::Error { .. } => (t.error, cx.t(&strings::SYNC_ERROR)),
    }
}

/// Place an absolutely positioned child on the side the close button belongs on.
trait CloseSide: Styled + Sized {
    fn map_close_side(self, cx: &App) -> Self {
        match chrome::close_side(cx) {
            Side::Left => self.left(px(10.0)),
            Side::Right => self.right(px(10.0)),
        }
    }
}

impl<T: Styled + Sized> CloseSide for T {}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn initials_of_names() {
        assert_eq!(initials("Zofia Wróblewska"), "ZW");
        assert_eq!(initials("łukasz nowak"), "ŁN");
        assert_eq!(initials("Cher"), "C");
        assert_eq!(initials(""), "");
        assert_eq!(initials("Anna Maria Kowalska"), "AM");
    }
}
