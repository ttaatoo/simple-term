//! Terminal settings for zed-terminal
//!
//! This module provides terminal configuration using a JSON config file.

use alacritty_terminal::vte::ansi::{
    CursorShape as AlacCursorShape, CursorStyle as AlacCursorStyle,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Alternate scroll mode
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlternateScroll {
    /// Alternate scroll enabled (default)
    #[default]
    On,
    /// Alternate scroll disabled
    Off,
}

/// Cursor shape
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    /// Cursor is a block like `█`.
    #[default]
    Block,
    /// Cursor is an underscore like `_`.
    Underline,
    /// Cursor is a vertical bar like `⎸`.
    Bar,
    /// Cursor is a hollow box like `▯`.
    Hollow,
}

impl From<CursorShape> for AlacCursorShape {
    fn from(value: CursorShape) -> Self {
        match value {
            CursorShape::Block => AlacCursorShape::Block,
            CursorShape::Underline => AlacCursorShape::Underline,
            CursorShape::Bar => AlacCursorShape::Beam,
            CursorShape::Hollow => AlacCursorShape::HollowBlock,
        }
    }
}

impl From<CursorShape> for AlacCursorStyle {
    fn from(value: CursorShape) -> Self {
        AlacCursorStyle {
            shape: value.into(),
            blinking: false,
        }
    }
}

/// Blinking behavior
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Blinking {
    /// Blinking off
    Off,
    /// Blinking controlled by terminal
    #[default]
    TerminalControlled,
    /// Blinking on
    On,
}

/// Line height setting
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LineHeight {
    /// Comfortable line height (1.618 ratio)
    #[default]
    Comfortable,
    /// Standard line height (1.3 ratio)
    Standard,
    /// Custom line height ratio
    Custom {
        /// Custom ratio value
        value: f32,
    },
}

impl LineHeight {
    pub fn to_ratio(&self) -> f32 {
        match self {
            LineHeight::Comfortable => 1.618,
            LineHeight::Standard => 1.3,
            LineHeight::Custom { value } => *value,
        }
    }
}

/// Working directory strategy
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WorkingDirectory {
    /// Use current file's directory
    #[default]
    CurrentFileDirectory,
    /// Use current project's directory
    CurrentProjectDirectory,
    /// Use first project's directory
    FirstProjectDirectory,
    /// Always use home directory
    AlwaysHome,
    /// Always use a specific directory
    Always {
        /// The directory path
        directory: PathBuf,
    },
}

/// Shell configuration
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ShellConfig {
    /// Use system default shell
    #[default]
    System,
    /// Use a specific program
    Program {
        /// Path to the shell program
        program: String,
    },
    /// Use a program with arguments
    WithArguments {
        /// Path to the shell program
        program: String,
        /// Arguments to pass to the shell
        args: Vec<String>,
    },
}

impl ShellConfig {
    pub fn to_shell(&self) -> super::Shell {
        match self {
            ShellConfig::System => super::Shell::System,
            ShellConfig::Program { program } => super::Shell::Program(program.clone()),
            ShellConfig::WithArguments { program, args } => super::Shell::WithArguments {
                program: program.clone(),
                args: args.clone(),
                title_override: None,
            },
        }
    }
}

/// Terminal settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminalSettings {
    /// Shell configuration
    #[serde(default)]
    pub shell: ShellConfig,
    /// Working directory strategy
    #[serde(default)]
    pub working_directory: WorkingDirectory,
    /// Font size in pixels
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// Font family name
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Font fallback family names (monospace preferred)
    #[serde(default = "default_font_fallbacks")]
    pub font_fallbacks: Vec<String>,
    /// Line height setting
    #[serde(default)]
    pub line_height: LineHeight,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Cursor shape
    #[serde(default)]
    pub cursor_shape: CursorShape,
    /// Blinking behavior
    #[serde(default)]
    pub blinking: Blinking,
    /// Alternate scroll mode
    #[serde(default)]
    pub alternate_scroll: AlternateScroll,
    /// Treat option key as meta
    #[serde(default)]
    pub option_as_meta: bool,
    /// Copy on select
    #[serde(default)]
    pub copy_on_select: bool,
    /// Keep selection after copy
    #[serde(default = "default_true")]
    pub keep_selection_on_copy: bool,
    /// Show terminal button in status bar
    #[serde(default = "default_true")]
    pub button: bool,
    /// Default terminal width
    #[serde(default = "default_width")]
    pub default_width: u32,
    /// Default terminal height
    #[serde(default = "default_height")]
    pub default_height: u32,
    /// Maximum scrollback lines
    #[serde(default = "default_scrollback")]
    pub max_scroll_history_lines: Option<usize>,
    /// Scroll multiplier
    #[serde(default = "default_scroll_multiplier")]
    pub scroll_multiplier: f32,
    /// Minimum contrast ratio
    #[serde(default = "default_minimum_contrast")]
    pub minimum_contrast: f32,
    /// Path hyperlink regex patterns
    #[serde(default)]
    pub path_hyperlink_regexes: Vec<String>,
    /// Path hyperlink timeout in milliseconds
    #[serde(default = "default_hyperlink_timeout")]
    pub path_hyperlink_timeout_ms: u64,
}

fn default_font_size() -> f32 {
    14.0
}

fn default_font_family() -> String {
    if cfg!(target_os = "macos") {
        "Menlo".to_string()
    } else if cfg!(target_os = "windows") {
        "Consolas".to_string()
    } else {
        "DejaVu Sans Mono".to_string()
    }
}

fn default_true() -> bool {
    true
}

fn default_font_fallbacks() -> Vec<String> {
    if cfg!(target_os = "macos") {
        vec![
            "SF Mono".to_string(),
            "Monaco".to_string(),
            "Courier".to_string(),
            "Noto Sans Mono".to_string(),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            "Cascadia Mono".to_string(),
            "Courier New".to_string(),
            "Lucida Console".to_string(),
            "Noto Sans Mono".to_string(),
        ]
    } else {
        vec![
            "Liberation Mono".to_string(),
            "Noto Sans Mono".to_string(),
            "Ubuntu Mono".to_string(),
            "Monospace".to_string(),
        ]
    }
}

fn default_width() -> u32 {
    640
}

fn default_height() -> u32 {
    320
}

fn default_scrollback() -> Option<usize> {
    Some(10_000)
}

fn default_scroll_multiplier() -> f32 {
    3.0
}

fn default_minimum_contrast() -> f32 {
    45.0
}

fn default_hyperlink_timeout() -> u64 {
    500
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: ShellConfig::default(),
            working_directory: WorkingDirectory::default(),
            font_size: default_font_size(),
            font_family: default_font_family(),
            font_fallbacks: default_font_fallbacks(),
            line_height: LineHeight::default(),
            env: HashMap::new(),
            cursor_shape: CursorShape::default(),
            blinking: Blinking::default(),
            alternate_scroll: AlternateScroll::default(),
            option_as_meta: false,
            copy_on_select: false,
            keep_selection_on_copy: true,
            button: true,
            default_width: default_width(),
            default_height: default_height(),
            max_scroll_history_lines: default_scrollback(),
            scroll_multiplier: default_scroll_multiplier(),
            minimum_contrast: default_minimum_contrast(),
            path_hyperlink_regexes: Vec::new(),
            path_hyperlink_timeout_ms: default_hyperlink_timeout(),
        }
    }
}

impl TerminalSettings {
    /// Load settings from a JSON file
    pub fn load(config_path: &PathBuf) -> Self {
        if config_path.exists() {
            match std::fs::read_to_string(config_path) {
                Ok(contents) => {
                    if let Ok(settings) = serde_json::from_str(&contents) {
                        return settings;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read config file: {}", e);
                }
            }
        }

        // Return defaults if file doesn't exist or parsing fails
        Self::default()
    }

    /// Get the config directory path
    pub fn config_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "zed-terminal", "ZedTerminal")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./config"))
    }

    /// Get the default config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }
}

#[cfg(test)]
mod tests {
    use super::{default_font_fallbacks, default_font_family, TerminalSettings};

    #[test]
    fn default_font_family_prefers_platform_monospace() {
        let family = default_font_family();
        #[cfg(target_os = "macos")]
        assert_eq!(family, "Menlo");
        #[cfg(target_os = "windows")]
        assert_eq!(family, "Consolas");
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        assert_eq!(family, "DejaVu Sans Mono");
    }

    #[test]
    fn default_font_fallbacks_are_present() {
        assert!(!default_font_fallbacks().is_empty());
    }

    #[test]
    fn terminal_settings_default_uses_fallback_list() {
        let settings = TerminalSettings::default();
        assert!(!settings.font_fallbacks.is_empty());
    }
}
