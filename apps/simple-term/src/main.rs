#![allow(unexpected_cfgs)]

//! Simple Term - A standalone terminal application

mod terminal_view;

#[cfg(target_os = "macos")]
mod macos;

use gpui::WindowHandle;
use gpui::{point, px};
#[cfg(not(target_os = "macos"))]
use gpui::{size, Bounds};
use gpui::{App, AppContext, Application, WindowBounds, WindowOptions};
#[cfg(target_os = "macos")]
use gpui::{TitlebarOptions, WindowKind};
use simple_term::TerminalSettings;
use terminal_view::TerminalView;

#[cfg(target_os = "macos")]
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
#[cfg(target_os = "macos")]
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) enum AppCommand {
    ToggleTerminal,
    TogglePinned,
    HideTerminal,
    UpdateHotkeys {
        global_hotkey: String,
        pin_hotkey: String,
    },
}

fn main() {
    env_logger::init();
    Application::new().run(|cx| {
        let settings = TerminalSettings::load(&TerminalSettings::config_path());

        #[cfg(target_os = "macos")]
        {
            run_macos_app(cx, settings);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = open_standard_window(cx, settings);
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn open_standard_window(cx: &mut App, settings: TerminalSettings) -> WindowHandle<TerminalView> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(
                px(settings.default_width as f32),
                px(settings.default_height as f32),
            ),
        })),
        ..Default::default()
    };

    cx.open_window(options, move |window, cx| {
        let settings = settings.clone();
        cx.new(move |cx| TerminalView::new(window, cx, settings, None, None))
    })
    .expect("Failed to open window")
}

#[cfg(target_os = "macos")]
fn run_macos_app(cx: &mut App, settings: TerminalSettings) {
    let (command_tx, command_rx) = smol::channel::unbounded::<AppCommand>();
    macos::set_command_sender(command_tx.clone());

    let controller = Rc::new(RefCell::new(AppShellController::new(
        settings,
        command_tx.clone(),
    )));
    controller.borrow_mut().bootstrap(cx);

    cx.spawn({
        let controller = controller.clone();
        let retry_command_tx = command_tx.clone();
        async move |async_cx| {
            while let Ok(command) = command_rx.recv().await {
                let command_for_retry = command.clone();
                let _ = async_cx.update(|cx| {
                    with_try_borrow_mut(
                        controller.as_ref(),
                        |controller| controller.handle_command(command, cx),
                        || {
                            log::warn!(
                                "app shell controller busy while handling {:?}; requeueing",
                                command_for_retry
                            );
                            let _ = retry_command_tx.try_send(command_for_retry);
                        },
                    );
                });
            }
        }
    })
    .detach();
}

#[cfg(target_os = "macos")]
fn with_try_borrow_mut<T, OnReady, OnBusy>(value: &RefCell<T>, on_ready: OnReady, on_busy: OnBusy)
where
    OnReady: FnOnce(&mut T),
    OnBusy: FnOnce(),
{
    if let Ok(mut borrowed) = value.try_borrow_mut() {
        on_ready(&mut borrowed);
    } else {
        on_busy();
    }
}

#[cfg(target_os = "macos")]
struct AppShellController {
    settings: TerminalSettings,
    command_tx: smol::channel::Sender<AppCommand>,
    terminal_window: Option<WindowHandle<TerminalView>>,
    visible: bool,
    pinned: bool,
    status_item: Option<macos::StatusItemHandle>,
    hotkey_manager: Option<GlobalHotKeyManager>,
}

#[cfg(target_os = "macos")]
impl AppShellController {
    fn new(settings: TerminalSettings, command_tx: smol::channel::Sender<AppCommand>) -> Self {
        Self {
            settings,
            command_tx,
            terminal_window: None,
            visible: false,
            pinned: false,
            status_item: None,
            hotkey_manager: None,
        }
    }

    fn bootstrap(&mut self, cx: &mut App) {
        macos::apply_app_activation_policy();
        self.status_item = macos::install_status_item(self.settings.button);
        self.install_global_hotkeys();
        self.show_terminal(cx);
    }

    fn install_global_hotkeys(&mut self) {
        // Drop prior registrations before applying updated shortcut bindings.
        self.hotkey_manager = None;

        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => manager,
            Err(err) => {
                log::warn!("failed to initialize global hotkey manager: {err}");
                return;
            }
        };

        let toggle_hotkey = Self::parse_hotkey_or_fallback(
            &self.settings.global_hotkey,
            Self::default_toggle_hotkey(),
            "global_hotkey",
        );
        let pin_hotkey = Self::parse_hotkey_or_fallback(
            &self.settings.pin_hotkey,
            Self::default_pin_hotkey(),
            "pin_hotkey",
        );

        let toggle_hotkey_id = match manager.register(toggle_hotkey) {
            Ok(_) => Some(toggle_hotkey.id()),
            Err(err) => {
                log::warn!(
                    "failed to register global_hotkey '{}': {err}",
                    toggle_hotkey
                );
                None
            }
        };

        let pin_hotkey_id = if toggle_hotkey.id() == pin_hotkey.id() {
            log::warn!(
                "pin_hotkey '{}' conflicts with global_hotkey '{}'; pin shortcut disabled",
                self.settings.pin_hotkey,
                self.settings.global_hotkey
            );
            None
        } else {
            match manager.register(pin_hotkey) {
                Ok(_) => Some(pin_hotkey.id()),
                Err(err) => {
                    log::warn!("failed to register pin_hotkey '{}': {err}", pin_hotkey);
                    None
                }
            }
        };

        if toggle_hotkey_id.is_none() && pin_hotkey_id.is_none() {
            return;
        }

        let command_tx = self.command_tx.clone();
        let _ = std::thread::Builder::new()
            .name("simple-term-hotkey-listener".to_string())
            .spawn(move || {
                let receiver = GlobalHotKeyEvent::receiver();
                while let Ok(event) = receiver.recv() {
                    if event.state != HotKeyState::Pressed {
                        continue;
                    }

                    if Some(event.id) == toggle_hotkey_id {
                        let _ = command_tx.try_send(AppCommand::ToggleTerminal);
                        continue;
                    }

                    if Some(event.id) == pin_hotkey_id {
                        let _ = command_tx.try_send(AppCommand::TogglePinned);
                    }
                }
            });

        self.hotkey_manager = Some(manager);
    }

    fn default_toggle_hotkey() -> HotKey {
        HotKey::new(Some(Modifiers::SUPER), Code::F4)
    }

    fn default_pin_hotkey() -> HotKey {
        HotKey::new(Some(Modifiers::SUPER), Code::Backquote)
    }

    fn parse_hotkey_or_fallback(configured_hotkey: &str, fallback: HotKey, label: &str) -> HotKey {
        if let Some(alias_hotkey) = Self::parse_reserved_or_alias_hotkey(configured_hotkey, label) {
            return alias_hotkey;
        }

        match configured_hotkey.parse::<HotKey>() {
            Ok(hotkey) => hotkey,
            Err(err) => {
                log::warn!(
                    "invalid {} '{}': {err}; falling back to {}",
                    label,
                    configured_hotkey,
                    fallback
                );
                fallback
            }
        }
    }

    fn parse_reserved_or_alias_hotkey(configured_hotkey: &str, label: &str) -> Option<HotKey> {
        if label != "global_hotkey" {
            return None;
        }

        let normalized = configured_hotkey
            .chars()
            .filter(|ch| !ch.is_ascii_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();

        match normalized.as_str() {
            "command+f5" | "cmd+f5" | "super+f5" | "meta+f5" => {
                let remapped = Self::default_toggle_hotkey();
                log::warn!(
                    "global_hotkey '{}' conflicts with macOS VoiceOver; remapping to {}",
                    configured_hotkey,
                    remapped
                );
                Some(remapped)
            }
            "cmd+r5" | "command+r5" | "super+r5" | "meta+r5" => Some(Self::default_toggle_hotkey()),
            _ => None,
        }
    }

    fn handle_command(&mut self, command: AppCommand, cx: &mut App) {
        match command {
            AppCommand::ToggleTerminal => {
                if self.visible {
                    self.hide_terminal(cx);
                } else {
                    self.show_terminal(cx);
                }
            }
            AppCommand::TogglePinned => self.toggle_terminal_pin(cx),
            AppCommand::HideTerminal => self.hide_terminal(cx),
            AppCommand::UpdateHotkeys {
                global_hotkey,
                pin_hotkey,
            } => {
                self.settings.global_hotkey = global_hotkey;
                self.settings.pin_hotkey = pin_hotkey;
                self.install_global_hotkeys();
            }
        }
    }

    fn toggle_terminal_pin(&mut self, cx: &mut App) {
        self.pinned = !self.pinned;
        if self.pinned {
            self.show_terminal(cx);
        }

        if let Some(window_handle) = self.terminal_window {
            let pinned = self.pinned;
            let _ = window_handle.update(cx, |_, window, _| {
                macos::set_window_pinned(window, pinned);
                if pinned {
                    window.activate_window();
                }
            });
        }
    }

    fn show_terminal(&mut self, cx: &mut App) {
        cx.activate(true);

        if let Some(window_handle) = self.terminal_window {
            let pinned = self.pinned;
            let updated = window_handle
                .update(cx, |_, window, _| {
                    macos::set_window_pinned(window, pinned);
                    window.activate_window();
                })
                .is_ok();

            if updated {
                self.visible = true;
                return;
            }

            self.terminal_window = None;
        }

        let placement = macos::resolve_panel_placement(
            self.settings.default_width as f32,
            self.settings.default_height as f32,
            self.settings.panel_top_inset,
        );

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(placement.bounds)),
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.0), px(12.0))),
            }),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: true,
            is_minimizable: true,
            ..Default::default()
        };

        let settings = self.settings.clone();
        let command_tx = self.command_tx.clone();
        let on_window_deactivated = Some(Arc::new(move || {
            let _ = command_tx.try_send(AppCommand::HideTerminal);
        }) as Arc<dyn Fn() + Send + Sync>);
        let command_tx = self.command_tx.clone();
        let on_hotkeys_updated = Some(Arc::new(move |global_hotkey: String, pin_hotkey: String| {
            let _ = command_tx.try_send(AppCommand::UpdateHotkeys {
                global_hotkey,
                pin_hotkey,
            });
        }) as Arc<dyn Fn(String, String) + Send + Sync>);

        match cx.open_window(options, move |window, cx| {
            let settings = settings.clone();
            let on_window_deactivated = on_window_deactivated.clone();
            let on_hotkeys_updated = on_hotkeys_updated.clone();
            cx.new(move |cx| {
                TerminalView::new(
                    window,
                    cx,
                    settings,
                    on_window_deactivated,
                    on_hotkeys_updated,
                )
            })
        }) {
            Ok(window_handle) => {
                let pinned = self.pinned;
                let _ = window_handle.update(cx, |_, window, _| {
                    macos::move_window_to(window, &placement);
                    macos::set_window_pinned(window, pinned);
                    window.activate_window();
                });
                self.terminal_window = Some(window_handle);
                self.visible = true;
            }
            Err(err) => {
                log::error!("failed to open terminal window: {err}");
                self.visible = false;
            }
        }
    }

    fn hide_terminal(&mut self, cx: &mut App) {
        if !Self::should_process_hide_terminal_request(
            self.visible,
            self.terminal_window.is_some(),
            self.pinned,
        ) {
            return;
        }

        cx.hide();
        self.visible = false;
    }

    fn should_process_hide_terminal_request(
        visible: bool,
        has_window_handle: bool,
        pinned: bool,
    ) -> bool {
        !pinned && (visible || has_window_handle)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::{with_try_borrow_mut, AppShellController};
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use std::cell::{Cell, RefCell};

    #[test]
    fn with_try_borrow_mut_runs_ready_path_when_cell_is_available() {
        let value = RefCell::new(5usize);
        let ready_called = Cell::new(false);
        let busy_called = Cell::new(false);

        with_try_borrow_mut(
            &value,
            |value| {
                *value += 1;
                ready_called.set(true);
            },
            || busy_called.set(true),
        );

        assert_eq!(*value.borrow(), 6);
        assert!(ready_called.get());
        assert!(!busy_called.get());
    }

    #[test]
    fn with_try_borrow_mut_runs_busy_path_when_cell_is_already_borrowed() {
        let value = RefCell::new(2usize);
        let _guard = value.borrow();
        let ready_called = Cell::new(false);
        let busy_called = Cell::new(false);

        with_try_borrow_mut(&value, |_| ready_called.set(true), || busy_called.set(true));

        assert!(!ready_called.get());
        assert!(busy_called.get());
    }

    #[test]
    fn hide_terminal_request_is_processed_when_visible_flag_is_false() {
        assert!(AppShellController::should_process_hide_terminal_request(
            false, true, false
        ));
        assert!(!AppShellController::should_process_hide_terminal_request(
            false, false, false
        ));
        assert!(!AppShellController::should_process_hide_terminal_request(
            true, true, true
        ));
    }

    #[test]
    fn parse_r5_alias_hotkey_maps_to_default_toggle_hotkey() {
        let expected = HotKey::new(Some(Modifiers::SUPER), Code::F4);
        assert_eq!(
            AppShellController::parse_hotkey_or_fallback("cmd+r5", expected, "global_hotkey"),
            expected
        );
    }

    #[test]
    fn parse_command_five_function_key_remaps_to_non_reserved_combo() {
        let expected = HotKey::new(Some(Modifiers::SUPER), Code::F4);
        assert_eq!(
            AppShellController::parse_hotkey_or_fallback("command+F5", expected, "global_hotkey"),
            expected
        );
    }
}
