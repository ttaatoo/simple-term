//! Terminal backend - wraps alacritty_terminal with PTY management

use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

use alacritty_terminal::event::{Event as AlacEvent, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
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

/// Event proxy that forwards alacritty events to a channel.
#[derive(Clone)]
pub struct EventProxy {
    sender: smol::channel::Sender<TerminalEvent>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: AlacEvent) {
        let event = match event {
            AlacEvent::Wakeup => TerminalEvent::Wakeup,
            AlacEvent::MouseCursorDirty => TerminalEvent::Wakeup,
            AlacEvent::CursorBlinkingChange => TerminalEvent::Wakeup,
            AlacEvent::Bell => TerminalEvent::Bell,
            AlacEvent::Title(title) => TerminalEvent::TitleChanged(title),
            AlacEvent::ChildExit(code) => TerminalEvent::Exit(code),
            _ => return,
        };
        let _ = self.sender.try_send(event);
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

        // TermSize is what Term::new and resize expect (implements Dimensions)
        let term_size = term::test::TermSize::new(
            window_size.num_cols as usize,
            window_size.num_lines as usize,
        );

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
            ..Default::default()
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
        let term_size = term::test::TermSize::new(
            window_size.num_cols as usize,
            window_size.num_lines as usize,
        );
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
    use super::{build_pty_env, EventProxy, TerminalEvent};
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
}
