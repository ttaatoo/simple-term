#![cfg(unix)]

use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use simple_term::alacritty_terminal::event::WindowSize;
use simple_term::alacritty_terminal::grid::Scroll;
use simple_term::alacritty_terminal::term::cell::Flags;
use simple_term::terminal::{Terminal, TerminalEvent};
use simple_term::Dimensions;
use simple_term::Shell;

const POLL_INTERVAL: Duration = Duration::from_millis(20);

fn window_size(lines: u16, cols: u16) -> WindowSize {
    WindowSize {
        num_lines: lines,
        num_cols: cols,
        cell_width: 1,
        cell_height: 1,
    }
}

fn spawn_terminal_script(script: &str, size: WindowSize, scrollback_lines: usize) -> Terminal {
    let shell = Shell::WithArguments {
        program: "/bin/sh".to_string(),
        args: vec!["-c".to_string(), script.to_string()],
        title_override: None,
    };

    Terminal::new(
        shell,
        std::env::current_dir().ok(),
        size,
        scrollback_lines,
        HashMap::new(),
    )
    .expect("failed to spawn terminal")
}

fn wait_until<F>(timeout: Duration, mut predicate: F) -> bool
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    loop {
        if predicate() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_event<F>(
    terminal: &Terminal,
    timeout: Duration,
    mut predicate: F,
) -> Option<TerminalEvent>
where
    F: FnMut(&TerminalEvent) -> bool,
{
    let start = Instant::now();
    loop {
        while let Ok(event) = terminal.events.try_recv() {
            if predicate(&event) {
                return Some(event);
            }
        }

        if start.elapsed() >= timeout {
            return None;
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn visible_screen_text(terminal: &Terminal) -> String {
    let term = terminal.term.lock();
    let content = term.renderable_content();
    let num_cols = term.columns();
    let num_lines = term.screen_lines();
    let display_offset = term.grid().display_offset();

    let mut rows = vec![vec![' '; num_cols]; num_lines];
    for indexed in content.display_iter {
        let row = indexed.point.line.0 + display_offset as i32;
        if row < 0 {
            continue;
        }

        let row_idx = row as usize;
        if row_idx >= num_lines {
            continue;
        }

        let col_idx = indexed.point.column.0;
        if col_idx >= num_cols || indexed.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }

        rows[row_idx][col_idx] = if indexed.cell.c == '\0' {
            ' '
        } else {
            indexed.cell.c
        };
    }

    rows.into_iter()
        .map(|row| row.into_iter().collect::<String>().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn wait_for_screen_contains(terminal: &Terminal, needle: &str, timeout: Duration) -> bool {
    wait_until(timeout, || visible_screen_text(terminal).contains(needle))
}

#[test]
fn paste_input_round_trips_through_pty() {
    let terminal = spawn_terminal_script(
        "IFS= read -r line; printf 'PASTE:%s\\n' \"$line\"; exit 0",
        window_size(24, 80),
        256,
    );

    terminal.write_str("hello pasted world\n");

    assert!(
        wait_for_screen_contains(
            &terminal,
            "PASTE:hello pasted world",
            Duration::from_secs(4)
        ),
        "expected paste output on terminal screen; screen:\n{}",
        visible_screen_text(&terminal)
    );

    assert!(matches!(
        wait_for_event(&terminal, Duration::from_secs(4), |event| {
            matches!(event, TerminalEvent::Exit(_))
        }),
        Some(TerminalEvent::Exit(0))
    ));
}

#[test]
fn resize_updates_pty_dimensions_for_subsequent_commands() {
    let terminal = spawn_terminal_script(
        "printf 'SIZE1:'; stty size; IFS= read -r _; printf ' SIZE2:'; stty size; exit 0",
        window_size(24, 80),
        128,
    );

    assert!(
        wait_for_screen_contains(&terminal, "SIZE1:24 80", Duration::from_secs(4)),
        "expected initial stty size output; screen:\n{}",
        visible_screen_text(&terminal)
    );

    terminal.resize(window_size(40, 100));
    terminal.write_str("\n");

    assert!(
        wait_for_screen_contains(&terminal, "SIZE2:40 100", Duration::from_secs(4)),
        "expected resized stty size output; screen:\n{}",
        visible_screen_text(&terminal)
    );

    assert!(matches!(
        wait_for_event(&terminal, Duration::from_secs(4), |event| {
            matches!(event, TerminalEvent::Exit(_))
        }),
        Some(TerminalEvent::Exit(0))
    ));
}

#[test]
fn scrollback_retains_emitted_lines_and_allows_history_navigation() {
    let terminal = spawn_terminal_script(
        "for i in $(seq 1 80); do printf 'LINE-%03d\\n' \"$i\"; done; sleep 0.1; exit 0",
        window_size(8, 80),
        512,
    );

    assert!(
        wait_for_screen_contains(&terminal, "LINE-080", Duration::from_secs(5)),
        "expected generated lines on terminal screen; screen:\n{}",
        visible_screen_text(&terminal)
    );

    {
        let term = terminal.term.lock();
        assert!(
            term.history_size() > 0,
            "expected non-zero history size after overflowing viewport"
        );
    }

    {
        let mut term = terminal.term.lock();
        term.scroll_display(Scroll::Top);
    }

    let top_text = visible_screen_text(&terminal);
    assert!(
        top_text.contains("LINE-001"),
        "expected oldest line in scrollback at top; screen:\n{}",
        top_text
    );

    {
        let mut term = terminal.term.lock();
        term.scroll_display(Scroll::Bottom);
    }

    let bottom_text = visible_screen_text(&terminal);
    assert!(
        bottom_text.contains("LINE-080"),
        "expected newest line after returning to bottom; screen:\n{}",
        bottom_text
    );
}

#[test]
fn emits_title_changed_event_from_osc_sequence() {
    let terminal = spawn_terminal_script(
        "printf '\\033]0;integration-title\\007'; sleep 0.05; exit 0",
        window_size(24, 80),
        128,
    );

    let title_event = wait_for_event(&terminal, Duration::from_secs(4), |event| {
        matches!(event, TerminalEvent::TitleChanged(_))
    });

    assert!(matches!(
        title_event,
        Some(TerminalEvent::TitleChanged(title)) if title == "integration-title"
    ));

    assert!(matches!(
        wait_for_event(&terminal, Duration::from_secs(4), |event| {
            matches!(event, TerminalEvent::Exit(_))
        }),
        Some(TerminalEvent::Exit(0))
    ));
}

#[test]
fn emits_exit_event_with_child_status() {
    let terminal = spawn_terminal_script("exit 17", window_size(24, 80), 64);

    let exit_event = wait_for_event(&terminal, Duration::from_secs(4), |event| {
        matches!(event, TerminalEvent::Exit(_))
    });

    assert!(matches!(exit_event, Some(TerminalEvent::Exit(17))));
}

#[test]
fn ctrl_c_produces_terminal_feedback_and_wakeup() {
    let terminal = spawn_terminal_script("cat", window_size(24, 80), 128);

    terminal.write(vec![0x03]); // Ctrl+C byte

    assert!(
        wait_for_screen_contains(&terminal, "^C", Duration::from_secs(4)),
        "expected terminal to echo caret notation for interrupt; screen:\\n{}",
        visible_screen_text(&terminal)
    );

    assert!(
        wait_for_event(&terminal, Duration::from_secs(4), |event| {
            matches!(event, TerminalEvent::Wakeup)
        })
        .is_some(),
        "expected wakeup event after Ctrl+C"
    );

    terminal.shutdown();
}
