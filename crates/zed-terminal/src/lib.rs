//! Zed Terminal - A standalone terminal emulator library
//!
//! This crate provides a terminal emulator based on alacritty_terminal,
//! adapted for use as a standalone library with GPUI.

pub mod mappings;
pub mod platform;
pub mod pty_info;
pub mod terminal;
pub mod terminal_hyperlinks;
pub mod terminal_settings;

pub use alacritty_terminal;

use serde::{Deserialize, Serialize};

pub use terminal_settings::{AlternateScroll, CursorShape, TerminalSettings};

/// Re-export commonly used types
pub use alacritty_terminal::{
    event::WindowSize,
    grid::Dimensions,
    index::{Column, Direction as AlacDirection, Line, Point as AlacPoint},
    selection::{Selection, SelectionRange, SelectionType},
    sync::FairMutex,
    term::{Config, RenderableCursor, TermMode},
    vte::ansi::{ClearMode, CursorStyle as AlacCursorStyle, Handler},
};

/// Terminal event types
pub mod events {
    /// Upward flowing events from terminal
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Event {
        TitleChanged,
        BreadcrumbsChanged,
        CloseTerminal,
        Bell,
        Wakeup,
        BlinkChanged(bool),
        SelectionsChanged,
        NewNavigationTarget(Option<super::MaybeNavigationTarget>),
        Open(super::MaybeNavigationTarget),
    }

    /// A string inside terminal, potentially useful as a URI
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum MaybeNavigationTarget {
        Url(String),
        PathLike(super::PathLikeTarget),
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PathLikeTarget {
        pub maybe_path: String,
        pub terminal_dir: Option<std::path::PathBuf>,
    }
}

pub use events::{Event, MaybeNavigationTarget, PathLikeTarget};

/// Terminal bounds and dimensions
pub mod bounds {
    use super::*;
    use alacritty_terminal::event::WindowSize;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TerminalBounds {
        pub cell_width: gpui::Pixels,
        pub line_height: gpui::Pixels,
        pub bounds: gpui::Bounds<gpui::Pixels>,
    }

    impl TerminalBounds {
        pub fn new(
            line_height: gpui::Pixels,
            cell_width: gpui::Pixels,
            bounds: gpui::Bounds<gpui::Pixels>,
        ) -> Self {
            TerminalBounds {
                cell_width,
                line_height,
                bounds,
            }
        }

        pub fn num_lines(&self) -> usize {
            (self.bounds.size.height / self.line_height).floor() as usize
        }

        pub fn num_columns(&self) -> usize {
            (self.bounds.size.width / self.cell_width).floor() as usize
        }

        pub fn height(&self) -> gpui::Pixels {
            self.bounds.size.height
        }

        pub fn width(&self) -> gpui::Pixels {
            self.bounds.size.width
        }

        pub fn cell_width(&self) -> gpui::Pixels {
            self.cell_width
        }

        pub fn line_height(&self) -> gpui::Pixels {
            self.line_height
        }
    }

    impl Default for TerminalBounds {
        fn default() -> Self {
            const DEBUG_TERMINAL_WIDTH: gpui::Pixels = gpui::px(500.);
            const DEBUG_TERMINAL_HEIGHT: gpui::Pixels = gpui::px(30.);
            const DEBUG_CELL_WIDTH: gpui::Pixels = gpui::px(5.);
            const DEBUG_LINE_HEIGHT: gpui::Pixels = gpui::px(5.);

            TerminalBounds::new(
                DEBUG_LINE_HEIGHT,
                DEBUG_CELL_WIDTH,
                gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: DEBUG_TERMINAL_WIDTH,
                        height: DEBUG_TERMINAL_HEIGHT,
                    },
                },
            )
        }
    }

    impl From<TerminalBounds> for WindowSize {
        fn from(val: TerminalBounds) -> Self {
            WindowSize {
                num_lines: val.num_lines() as u16,
                num_cols: val.num_columns() as u16,
                cell_width: f32::from(val.cell_width()) as u16,
                cell_height: f32::from(val.line_height()) as u16,
            }
        }
    }

    impl Dimensions for TerminalBounds {
        fn total_lines(&self) -> usize {
            self.screen_lines()
        }

        fn screen_lines(&self) -> usize {
            self.num_lines()
        }

        fn columns(&self) -> usize {
            self.num_columns()
        }
    }
}

pub use bounds::TerminalBounds;

/// Configuration constants
pub mod config {
    pub const DEFAULT_SCROLL_HISTORY_LINES: usize = 10_000;
    pub const MAX_SCROLL_HISTORY_LINES: usize = 100_000;
}

/// Terminal error types
pub mod error {
    use std::path::PathBuf;

    #[derive(Debug)]
    pub struct TerminalError {
        pub directory: Option<PathBuf>,
        pub program: Option<String>,
        pub args: Option<Vec<String>>,
        pub title_override: Option<String>,
        pub source: std::io::Error,
    }

    impl std::fmt::Display for TerminalError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let dir_string = self
                .directory
                .clone()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| "<none specified>".to_string());

            let shell = format!(
                "{} {}",
                self.program.as_deref().unwrap_or("<system defined shell>"),
                self.args.as_ref().map(|a| a.join(" ")).unwrap_or_default()
            );

            write!(
                f,
                "Working directory: {} Shell command: `{}`, IOError: {}",
                dir_string, shell, self.source
            )
        }
    }

    impl std::error::Error for TerminalError {}
}

/// Shell configuration
#[derive(Debug, Clone)]
pub enum Shell {
    System,
    Program(String),
    WithArguments {
        program: String,
        args: Vec<String>,
        title_override: Option<String>,
    },
}

impl Default for Shell {
    fn default() -> Self {
        Shell::System
    }
}

/// Path style for hyperlinks
#[derive(Debug, Clone, Copy, Default)]
pub enum PathStyle {
    #[default]
    Unix,
    Windows,
    Remote,
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size, Bounds};

    #[test]
    fn terminal_bounds_reports_dimensions_consistently() {
        let bounds = TerminalBounds::new(
            px(10.0),
            px(5.0),
            Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(55.0), px(26.0)),
            },
        );

        assert_eq!(bounds.num_columns(), 11);
        assert_eq!(bounds.num_lines(), 2);
        assert_eq!(bounds.columns(), 11);
        assert_eq!(bounds.screen_lines(), 2);
        assert_eq!(bounds.total_lines(), 2);
    }

    #[test]
    fn terminal_bounds_converts_to_window_size() {
        let bounds = TerminalBounds::new(
            px(12.0),
            px(7.0),
            Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(70.0), px(36.0)),
            },
        );

        let ws: WindowSize = bounds.into();
        assert_eq!(ws.num_cols, 10);
        assert_eq!(ws.num_lines, 3);
        assert_eq!(ws.cell_width, 7);
        assert_eq!(ws.cell_height, 12);
    }

    #[test]
    fn shell_default_is_system() {
        assert!(matches!(Shell::default(), Shell::System));
    }
}
