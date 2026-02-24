#![cfg(unix)]

use std::collections::HashMap;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use simple_term::alacritty_terminal::{event::WindowSize, tty};
use simple_term::pty_info::{ProcessInfo, PtyProcessInfo};

fn spawn_sleep_pty(seconds: u64) -> tty::Pty {
    let options = tty::Options {
        shell: Some(tty::Shell::new(
            "/bin/sleep".to_string(),
            vec![seconds.to_string()],
        )),
        working_directory: std::env::current_dir().ok(),
        drain_on_exit: false,
        env: HashMap::new(),
    };

    tty::new(
        &options,
        WindowSize {
            num_lines: 24,
            num_cols: 80,
            cell_width: 1,
            cell_height: 1,
        },
        0,
    )
    .expect("failed to spawn test PTY")
}

fn wait_for_info<F>(timeout: Duration, mut f: F) -> Option<ProcessInfo>
where
    F: FnMut() -> Option<ProcessInfo>,
{
    let start = Instant::now();
    loop {
        if let Some(info) = f() {
            return Some(info);
        }
        if start.elapsed() >= timeout {
            return None;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_child_exit(child_pid: i32, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        let mut status = 0;
        let wait_result = unsafe { libc::waitpid(child_pid, &mut status, libc::WNOHANG) };
        if wait_result == child_pid {
            return true;
        }
        if wait_result == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD) {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_no_info<F>(timeout: Duration, mut f: F) -> bool
where
    F: FnMut() -> Option<ProcessInfo>,
{
    let start = Instant::now();
    loop {
        if f().is_none() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn load_populates_process_info_and_updates_current_snapshot() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);

    assert!(process_info.get_current().is_none());

    let loaded = wait_for_info(Duration::from_secs(2), || process_info.load())
        .expect("expected process info for running PTY child");

    assert!(!loaded.name.is_empty());
    assert!(!loaded.argv.is_empty());
    assert!(
        loaded.argv.iter().any(|arg| arg == "30"),
        "expected argv to include sleep duration, got: {:?}",
        loaded.argv
    );
    assert_eq!(
        process_info
            .get_current()
            .as_ref()
            .map(|info| info.name.clone()),
        Some(loaded.name.clone())
    );

    assert!(process_info.pid().is_some());
    assert!(process_info.pid_getter().fallback_pid().as_u32() > 0);
}

#[test]
fn update_returns_latest_snapshot_and_sets_current() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);

    let updated = wait_for_info(Duration::from_secs(2), || process_info.update())
        .expect("expected update to return process info");

    let current = process_info
        .get_current()
        .expect("expected current process info to be set");
    assert_eq!(updated.name, current.name);
    assert_eq!(updated.cwd, current.cwd);
    assert_eq!(updated.argv, current.argv);
}

#[test]
fn kill_child_process_terminates_running_pty_child() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);

    let _loaded = wait_for_info(Duration::from_secs(2), || process_info.load())
        .expect("expected process info before kill");

    let child_pid = process_info.pid_getter().fallback_pid().as_u32() as i32;
    assert!(process_info.kill_child_process());

    let became_reaped = wait_for_child_exit(child_pid, Duration::from_secs(3));
    assert!(
        became_reaped,
        "expected child process to exit and be observable via waitpid"
    );
}

#[test]
fn kill_current_process_terminates_foreground_process_group() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);

    let _loaded = wait_for_info(Duration::from_secs(2), || process_info.load())
        .expect("expected process info before kill");

    let child_pid = process_info.pid_getter().fallback_pid().as_u32() as i32;
    assert!(process_info.kill_current_process());

    let became_reaped = wait_for_child_exit(child_pid, Duration::from_secs(3));
    assert!(
        became_reaped,
        "expected foreground process group to exit after kill_current_process"
    );
}

#[test]
fn load_after_exit_returns_none_and_update_clears_cached_snapshot() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);

    let _initial = wait_for_info(Duration::from_secs(2), || process_info.load())
        .expect("expected initial process info");
    assert!(process_info.get_current().is_some());

    let child_pid = process_info.pid_getter().fallback_pid().as_u32() as i32;
    assert!(process_info.kill_child_process());
    assert!(wait_for_child_exit(child_pid, Duration::from_secs(3)));

    assert!(
        wait_for_no_info(Duration::from_secs(2), || process_info.load()),
        "expected load() to eventually return None after exit"
    );
    assert!(
        process_info.get_current().is_some(),
        "load() should not clear cached snapshot on failure"
    );

    assert!(process_info.update().is_none());
    assert!(process_info.get_current().is_none());
}

#[test]
fn pid_falls_back_to_child_pid_when_tty_fd_is_closed() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);
    let fallback_pid = process_info.pid_getter().fallback_pid();

    drop(pty);

    assert_eq!(process_info.pid(), Some(fallback_pid));
}

#[test]
fn kill_child_process_returns_false_after_process_is_gone() {
    let pty = spawn_sleep_pty(30);
    let process_info = PtyProcessInfo::new(&pty);

    let child_pid = process_info.pid_getter().fallback_pid().as_u32() as i32;
    assert!(process_info.kill_child_process());
    assert!(wait_for_child_exit(child_pid, Duration::from_secs(3)));

    assert!(process_info.update().is_none());
    assert!(!process_info.kill_child_process());
}
