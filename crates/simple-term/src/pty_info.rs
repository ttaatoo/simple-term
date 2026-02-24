//! PTY process information for simple-term

use alacritty_terminal::tty::Pty;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::num::NonZeroU32;

#[cfg(unix)]
use std::os::fd::AsRawFd;

#[cfg(target_os = "windows")]
use windows::Win32::{Foundation::HANDLE, System::Threading::GetProcessId};

use sysinfo::{Pid, Process, ProcessRefreshKind, RefreshKind, System, UpdateKind};

#[derive(Clone, Copy)]
pub struct ProcessIdGetter {
    handle: i32,
    fallback_pid: u32,
}

impl ProcessIdGetter {
    pub fn fallback_pid(&self) -> Pid {
        Pid::from_u32(self.fallback_pid)
    }
}

#[cfg(unix)]
impl ProcessIdGetter {
    pub fn new(pty: &Pty) -> ProcessIdGetter {
        ProcessIdGetter {
            handle: pty.file().as_raw_fd(),
            fallback_pid: pty.child().id(),
        }
    }

    pub fn pid(&self) -> Option<Pid> {
        let pid = unsafe { libc::tcgetpgrp(self.handle) };
        if pid < 0 {
            return Some(Pid::from_u32(self.fallback_pid));
        }
        Some(Pid::from_u32(pid as u32))
    }
}

#[cfg(windows)]
impl ProcessIdGetter {
    pub fn new(pty: &Pty) -> ProcessIdGetter {
        let child = pty.child_watcher();
        let handle = child.raw_handle();
        let fallback_pid = child.pid().unwrap_or_else(|| unsafe {
            NonZeroU32::new_unchecked(GetProcessId(HANDLE(handle as _)))
        });

        ProcessIdGetter {
            handle: handle as i32,
            fallback_pid: u32::from(fallback_pid),
        }
    }

    pub fn pid(&self) -> Option<Pid> {
        let pid = unsafe { GetProcessId(HANDLE(self.handle as _)) };
        if pid == 0 {
            if self.fallback_pid == 0 {
                return None;
            }
            return Some(Pid::from_u32(self.fallback_pid));
        }
        Some(Pid::from_u32(pid))
    }
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub name: String,
    pub cwd: PathBuf,
    pub argv: Vec<String>,
}

/// Fetches Pseudo-Terminal (PTY) process information
pub struct PtyProcessInfo {
    system: RwLock<System>,
    refresh_kind: ProcessRefreshKind,
    pid_getter: ProcessIdGetter,
    pub current: RwLock<Option<ProcessInfo>>,
}

impl PtyProcessInfo {
    pub fn new(pty: &Pty) -> PtyProcessInfo {
        let process_refresh_kind = ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always);
        let refresh_kind = RefreshKind::new().with_processes(process_refresh_kind);
        let system = System::new_with_specifics(refresh_kind);

        PtyProcessInfo {
            system: RwLock::new(system),
            refresh_kind: process_refresh_kind,
            pid_getter: ProcessIdGetter::new(pty),
            current: RwLock::new(None),
        }
    }

    pub fn pid_getter(&self) -> &ProcessIdGetter {
        &self.pid_getter
    }

    fn refresh(&self) -> Option<MappedRwLockReadGuard<'_, Process>> {
        let pid = self.pid_getter.pid()?;
        if self.system.write().refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            self.refresh_kind,
        ) == 1
        {
            RwLockReadGuard::try_map(self.system.read(), |system| system.process(pid)).ok()
        } else {
            None
        }
    }

    fn get_child(&self) -> Option<MappedRwLockReadGuard<'_, Process>> {
        let pid = self.pid_getter.fallback_pid();
        RwLockReadGuard::try_map(self.system.read(), |system| system.process(pid)).ok()
    }

    #[cfg(unix)]
    pub fn kill_current_process(&self) -> bool {
        let Some(pid) = self.pid_getter.pid() else {
            return false;
        };
        unsafe { libc::killpg(pid.as_u32() as i32, libc::SIGKILL) == 0 }
    }

    #[cfg(not(unix))]
    pub fn kill_current_process(&self) -> bool {
        self.refresh().is_some_and(|process| process.kill())
    }

    pub fn kill_child_process(&self) -> bool {
        self.get_child().is_some_and(|process| process.kill())
    }

    pub fn load(&self) -> Option<ProcessInfo> {
        let process = self.refresh()?;
        let cwd = process.cwd().map_or(PathBuf::new(), |p| p.to_owned());

        let info = ProcessInfo {
            name: process.name().to_str()?.to_owned(),
            cwd,
            argv: process
                .cmd()
                .iter()
                .filter_map(|s| s.to_str().map(ToOwned::to_owned))
                .collect(),
        };
        *self.current.write() = Some(info.clone());
        Some(info)
    }

    pub fn update(&self) -> Option<ProcessInfo> {
        let current = self.load();
        *self.current.write() = current.clone();
        current
    }

    pub fn get_current(&self) -> Option<ProcessInfo> {
        self.current.read().clone()
    }

    pub fn pid(&self) -> Option<Pid> {
        self.pid_getter.pid()
    }
}
