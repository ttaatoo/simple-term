//! Terminal backend - wraps alacritty_terminal with PTY management

use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

use alacritty_terminal::event::{Event as AlacEvent, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{self, Term};
use alacritty_terminal::tty;

use crate::Shell;

/// Events sent from the terminal backend to the UI layer.
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    Wakeup,
    Bell,
    TitleChanged(String),
    Exit(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackpressurePolicy {
    DropWhenFull,
    KeepLatestWhenFull,
}

fn backpressure_policy(event: &TerminalEvent) -> BackpressurePolicy {
    match event {
        TerminalEvent::Wakeup | TerminalEvent::Bell => BackpressurePolicy::DropWhenFull,
        TerminalEvent::TitleChanged(_) | TerminalEvent::Exit(_) => {
            BackpressurePolicy::KeepLatestWhenFull
        }
    }
}

fn map_event(event: AlacEvent) -> Option<TerminalEvent> {
    match event {
        AlacEvent::Wakeup => Some(TerminalEvent::Wakeup),
        AlacEvent::MouseCursorDirty => Some(TerminalEvent::Wakeup),
        AlacEvent::CursorBlinkingChange => Some(TerminalEvent::Wakeup),
        AlacEvent::Bell => Some(TerminalEvent::Bell),
        AlacEvent::Title(title) => Some(TerminalEvent::TitleChanged(title)),
        AlacEvent::ChildExit(code) => Some(TerminalEvent::Exit(code)),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TerminalDimensions {
    cols: usize,
    lines: usize,
}

impl Dimensions for TerminalDimensions {
    fn total_lines(&self) -> usize {
        self.lines
    }

    fn screen_lines(&self) -> usize {
        self.lines
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

fn terminal_dimensions(window_size: WindowSize) -> TerminalDimensions {
    TerminalDimensions {
        cols: window_size.num_cols as usize,
        lines: window_size.num_lines as usize,
    }
}

/// Event proxy that forwards alacritty events to a channel.
#[derive(Clone)]
pub struct EventProxy {
    sender: smol::channel::Sender<TerminalEvent>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: AlacEvent) {
        let Some(event) = map_event(event) else {
            return;
        };

        match self.sender.try_send(event) {
            Ok(()) => {}
            Err(smol::channel::TrySendError::Full(event)) => {
                if matches!(
                    backpressure_policy(&event),
                    BackpressurePolicy::KeepLatestWhenFull
                ) {
                    let _ = self.sender.force_send(event);
                }
            }
            Err(smol::channel::TrySendError::Closed(_)) => {}
        }
    }
}

/// The terminal backend managing PTY, event loop, and terminal state.
pub struct Terminal {
    /// Thread-safe access to the terminal state.
    pub term: Arc<FairMutex<Term<EventProxy>>>,
    /// Channel to send input/resize commands to the PTY.
    sender: EventLoopSender,
    /// Receiver for terminal events (wakeup, bell, title changes, etc.).
    pub events: smol::channel::Receiver<TerminalEvent>,
    /// Handle to the event loop thread.
    _event_loop_handle: JoinHandle<(
        EventLoop<tty::Pty, EventProxy>,
        alacritty_terminal::event_loop::State,
    )>,
}

impl Terminal {
    /// Spawn a new terminal with the given shell and window size.
    pub fn new(
        shell: Shell,
        working_directory: Option<PathBuf>,
        window_size: WindowSize,
        scrollback_lines: usize,
        environment: HashMap<String, String>,
    ) -> io::Result<Self> {
        let (event_sender, event_receiver) = smol::channel::bounded(256);
        let event_proxy = EventProxy {
            sender: event_sender,
        };

        // Configure the terminal emulator
        let config = term::Config {
            scrolling_history: scrollback_lines.min(crate::config::MAX_SCROLL_HISTORY_LINES),
            ..Default::default()
        };

        let term_size = terminal_dimensions(window_size);

        // Create the terminal state
        let term = Term::new(config, &term_size, event_proxy.clone());
        let term = Arc::new(FairMutex::new(term));

        // Configure PTY options
        let shell_option = match &shell {
            Shell::System => None,
            Shell::Program(prog) => Some(tty::Shell::new(prog.clone(), vec![])),
            Shell::WithArguments { program, args, .. } => {
                Some(tty::Shell::new(program.clone(), args.clone()))
            }
        };

        let env = build_pty_env(&environment);

        let pty_options = tty::Options {
            shell: shell_option,
            working_directory,
            drain_on_exit: false,
            env,
        };

        // Spawn the PTY
        let pty = tty::new(&pty_options, window_size, 0)?;

        // Create and spawn the event loop
        let event_loop = EventLoop::new(term.clone(), event_proxy, pty, false, false)?;

        let sender = event_loop.channel();
        let handle = event_loop.spawn();

        Ok(Terminal {
            term,
            sender,
            events: event_receiver,
            _event_loop_handle: handle,
        })
    }

    /// Write bytes to the PTY.
    pub fn write(&self, data: impl Into<Cow<'static, [u8]>>) {
        let _ = self.sender.send(Msg::Input(data.into()));
    }

    /// Write a string to the PTY.
    pub fn write_str(&self, s: &str) {
        self.write(s.as_bytes().to_vec());
    }

    /// Resize the terminal.
    pub fn resize(&self, window_size: WindowSize) {
        let term_size = terminal_dimensions(window_size);
        // Resize the PTY first via event loop
        let _ = self.sender.send(Msg::Resize(window_size));
        // Then resize the terminal grid
        self.term.lock().resize(term_size);
    }

    /// Shutdown the terminal.
    pub fn shutdown(&self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn build_pty_env(extra_env: &HashMap<String, String>) -> HashMap<String, String> {
    let mut env = extra_env.clone();
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env
}

#[cfg(test)]
mod tests {
    use super::{
        build_pty_env, terminal_dimensions, EventProxy, TerminalDimensions, TerminalEvent,
    };
    use alacritty_terminal::event::{Event as AlacEvent, EventListener};
    use std::collections::HashMap;

    #[test]
    fn pty_env_includes_default_term() {
        let env = build_pty_env(&HashMap::new());
        assert_eq!(env.get("TERM").map(String::as_str), Some("xterm-256color"));
    }

    #[test]
    fn pty_env_preserves_custom_entries() {
        let mut extra = HashMap::new();
        extra.insert("FOO".to_string(), "BAR".to_string());

        let env = build_pty_env(&extra);
        assert_eq!(env.get("FOO").map(String::as_str), Some("BAR"));
    }

    #[test]
    fn pty_env_keeps_terminal_type_consistent() {
        let mut extra = HashMap::new();
        extra.insert("TERM".to_string(), "vt100".to_string());

        let env = build_pty_env(&extra);
        assert_eq!(env.get("TERM").map(String::as_str), Some("xterm-256color"));
    }

    #[test]
    fn terminal_dimensions_match_window_size() {
        let window_size = alacritty_terminal::event::WindowSize {
            num_lines: 40,
            num_cols: 120,
            cell_width: 9,
            cell_height: 18,
        };

        let dims = terminal_dimensions(window_size);
        assert_eq!(
            dims,
            TerminalDimensions {
                cols: 120,
                lines: 40
            }
        );
    }

    #[test]
    fn event_proxy_maps_mouse_cursor_dirty_to_wakeup() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::MouseCursorDirty);

        assert!(matches!(receiver.try_recv(), Ok(TerminalEvent::Wakeup)));
    }

    #[test]
    fn event_proxy_maps_cursor_blinking_change_to_wakeup() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::CursorBlinkingChange);

        assert!(matches!(receiver.try_recv(), Ok(TerminalEvent::Wakeup)));
    }

    #[test]
    fn event_proxy_maps_wakeup_bell_title_and_exit_events() {
        let (sender, receiver) = smol::channel::bounded(8);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::Wakeup);
        proxy.send_event(AlacEvent::Bell);
        proxy.send_event(AlacEvent::Title("shell".to_string()));
        proxy.send_event(AlacEvent::ChildExit(42));

        assert!(matches!(receiver.try_recv(), Ok(TerminalEvent::Wakeup)));
        assert!(matches!(receiver.try_recv(), Ok(TerminalEvent::Bell)));
        assert!(matches!(
            receiver.try_recv(),
            Ok(TerminalEvent::TitleChanged(title)) if title == "shell"
        ));
        assert!(matches!(receiver.try_recv(), Ok(TerminalEvent::Exit(42))));
    }

    #[test]
    fn event_proxy_ignores_unmapped_events() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::ResetTitle);

        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn event_proxy_keeps_non_wakeup_events_when_channel_is_full() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::Wakeup);
        proxy.send_event(AlacEvent::Title("shell".to_string()));

        let mut saw_title = false;
        while let Ok(event) = receiver.try_recv() {
            if matches!(event, TerminalEvent::TitleChanged(title) if title == "shell") {
                saw_title = true;
            }
        }

        assert!(
            saw_title,
            "title event should not be dropped when queue is full"
        );
    }

    #[test]
    fn event_proxy_drops_wakeup_when_channel_is_full() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::Title("sticky".to_string()));
        proxy.send_event(AlacEvent::Wakeup);

        assert!(matches!(
            receiver.try_recv(),
            Ok(TerminalEvent::TitleChanged(title)) if title == "sticky"
        ));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn event_proxy_drops_bell_when_channel_is_full() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::Wakeup);
        proxy.send_event(AlacEvent::Bell);

        assert!(matches!(receiver.try_recv(), Ok(TerminalEvent::Wakeup)));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn event_proxy_keeps_exit_when_channel_is_full() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::Wakeup);
        proxy.send_event(AlacEvent::ChildExit(7));

        let mut saw_exit = false;
        while let Ok(event) = receiver.try_recv() {
            if matches!(event, TerminalEvent::Exit(7)) {
                saw_exit = true;
            }
        }

        assert!(
            saw_exit,
            "exit event should not be dropped when queue is full"
        );
    }

    #[test]
    fn event_proxy_keeps_latest_title_when_channel_is_full() {
        let (sender, receiver) = smol::channel::bounded(1);
        let proxy = EventProxy { sender };

        proxy.send_event(AlacEvent::Wakeup);
        proxy.send_event(AlacEvent::Title("first".to_string()));
        proxy.send_event(AlacEvent::Title("latest".to_string()));

        assert!(matches!(
            receiver.try_recv(),
            Ok(TerminalEvent::TitleChanged(title)) if title == "latest"
        ));
        assert!(receiver.try_recv().is_err());
    }
}
